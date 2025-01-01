#![no_main]
#![no_std]
#![feature(sync_unsafe_cell)]
#![allow(static_mut_refs)]

use alloc::{
    boxed::Box,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use core::{
    self,
    ptr,
    slice,
};

use anyhow::{
    anyhow,
    Context,
    Error,
};
use hook::{
    Function,
    StaticHook,
    TrampolineHook,
};
use image_info::{
    ImageBuffer,
    ImageInfo,
};
use obfstr::obfstr;
use pelite::{
    PeFile,
    PeView,
    Wrap,
};
use signature::Signature;
use uefi::{
    prelude::*,
    proto::{
        console::text::Color,
        device_path::{
            build::{
                self,
            },
            text::{
                AllowShortcuts,
                DisplayOnly,
            },
            DevicePath,
        },
        media::{
            file::{
                File,
                FileAttribute,
                FileMode,
            },
            fs::SimpleFileSystem,
        },
    },
    table::{
        boot::{
            LoadImageSource,
            OpenProtocolAttributes,
            OpenProtocolParams,
            SearchType,
        },
        SystemTable,
    },
    CStr16,
    Identify,
};
use uefi_core::system_table;
use utils::include_bytes_align_as;
use wdef::{
    BlImgAllocateImageBuffer,
    ImgArchStartBootApplication,
    LoaderParameterBlock,
    OslFwpKernelSetupPhase1,
    BL_MEMORY_TYPE_DRIVER,
    KLDR_DATA_TABLE_ENTRY,
    NT_STATUS,
};

use crate::{
    uefi_core::{
        enter_execution_context,
        ExecutionContext,
    },
    utils::{
        show_select,
        press_enter_to_continue,
        set_exit_boot_services,
    },
};

extern crate alloc;

const WINDOWS_BOOTMGR_PATH: &'static [u16] =
    obfstr::wide!("\\efi\\microsoft\\boot\\bootmgfw.efi\0");

type FnExitBootServices =
    unsafe extern "efiapi" fn(image_handle: uefi_raw::Handle, map_key: usize) -> Status;

#[repr(align(4096))]
struct Align4096;

static TARGET_DRIVER: &'static [u8] =
    include_bytes_align_as!(Align4096, r"../driver/driver_uefi.dll");

type StaticTrampolineHook<H> = StaticHook<H, TrampolineHook<H>>;

pub static mut HOOK_IMG_ARCH_START_BOOT_APPLICATION: StaticTrampolineHook<
    ImgArchStartBootApplication,
> = StaticHook::new();
pub static mut HOOK_BL_IMG_ALLOCATE_IMAGE_BUFFER: StaticTrampolineHook<BlImgAllocateImageBuffer> =
    StaticHook::new();
pub static mut HOOK_OSL_FWP_KERNEL_SETUP_PHASE1: StaticTrampolineHook<OslFwpKernelSetupPhase1> =
    StaticHook::new();

/* Called from the boot manager */
extern "efiapi" fn hooked_img_arch_start_boot_application(
    app_entry: *const (),
    image_base: *mut u8,
    image_size: u32,
    boot_option: u8,
    return_arguments: *mut (),
) -> u32 {
    let _exec_guard = enter_execution_context(ExecutionContext::WINBOOTMGR);

    // TODO: Check what image we're loading
    let original = unsafe {
        HOOK_IMG_ARCH_START_BOOT_APPLICATION.disable();
        HOOK_IMG_ARCH_START_BOOT_APPLICATION
            .target()
            .unwrap_unchecked()
    };

    let winload = ImageInfo {
        image_base,
        image_size: image_size as usize,
    };
    if let Err(err) = setup_hooks_winload(winload) {
        log::error!("{:#}", err);
        utils::press_enter_to_continue();
    }

    log::debug!("Calling ImgArchStartBootApplication");
    let result = original(
        app_entry,
        image_base,
        image_size,
        boot_option,
        return_arguments,
    );

    /* If this returns, we're in some kind of recovery mode... */
    log::debug!("Called original ImgArchStartBootApplication");

    result
}

/* Called in WinLoad context */
extern "efiapi" fn hooked_bl_img_allocate_image_buffer(
    image_buffer: *mut *mut u8,
    image_size: usize,
    memory_type: u32,
    attributes: u32,
    unused: u64,
    flags: u32,
) -> NT_STATUS {
    let _guard = enter_execution_context(ExecutionContext::WINLOAD);
    let original = unsafe {
        HOOK_BL_IMG_ALLOCATE_IMAGE_BUFFER.disable();
        HOOK_BL_IMG_ALLOCATE_IMAGE_BUFFER
            .target()
            .unwrap_unchecked()
    };

    // log::debug!(
    //     "BlMemory size: {:X}, type: {:X}, attr: {:X}, unused: {:X}, flags: {:X}",
    //     image_size,
    //     memory_type,
    //     attributes,
    //     unused,
    //     flags
    // );
    let original_result = original(
        image_buffer,
        image_size,
        memory_type,
        attributes,
        unused,
        flags,
    );

    let image_buffer = unsafe { &mut IMAGE_BUFFER };
    if image_buffer.is_some() {
        /* We already have an image buffer for some reason... */
        return original_result;
    }

    if memory_type != BL_MEMORY_TYPE_DRIVER {
        /* Allocation wasnt a driver, we'll wait untill the bootloader tried to allocate a driver buffer */
        unsafe {
            HOOK_BL_IMG_ALLOCATE_IMAGE_BUFFER.enable(hooked_bl_img_allocate_image_buffer);
        };
        return original_result;
    }

    match allocate_image_buffer(original, memory_type, attributes, unused, flags) {
        Ok(buffer) => {
            *image_buffer = Some(buffer);
        }
        Err(err) => {
            log::error!("{}: {:?}", obfstr!("Failed to allocate image buffer"), err);
        }
    }

    original_result
}

fn allocate_image_buffer(
    bl_allocate: BlImgAllocateImageBuffer,
    memory_type: u32,
    attributes: u32,
    unused: u64,
    flags: u32,
) -> anyhow::Result<ImageBuffer> {
    let pe = PeFile::from_bytes(TARGET_DRIVER)
        .map_err(|err| anyhow!("{}: {}", obfstr!("failed to parse packed driver"), err))?;

    let mut image_buffer = ptr::null_mut();
    let image_size = match pe.optional_header() {
        Wrap::T32(header) => header.SizeOfImage,
        Wrap::T64(header) => header.SizeOfImage,
    } as usize;

    log::debug!("{}: {}", obfstr!("Packed driver image size is"), image_size);
    let status = bl_allocate(
        &mut image_buffer,
        image_size,
        memory_type,
        attributes,
        unused,
        flags,
    );

    if status != 0 {
        anyhow::bail!("NT error {:X}", status)
    }

    log::debug!(
        "Allocated packed driver image buffer at {:X}",
        image_buffer as u64
    );
    Ok(ImageBuffer {
        address: image_buffer,
        length: image_size,
    })
}

extern "efiapi" fn hooked_osl_fwp_kernel_setup_phase1(lpb: *mut LoaderParameterBlock) -> u32 {
    let _exec_guard = enter_execution_context(ExecutionContext::WINLOAD);

    let original = unsafe {
        HOOK_OSL_FWP_KERNEL_SETUP_PHASE1.disable();
        HOOK_OSL_FWP_KERNEL_SETUP_PHASE1.target().unwrap_unchecked()
    };

    unsafe {
        MAPPING_RESULT = Some(handle_osl_lpb(lpb));
    }

    original(lpb)
}

static mut ORIGINAL_EXIT_BOOT_SERVICES: Option<FnExitBootServices> = None;

static mut WINLOAD_IMAGE: Option<ImageInfo> = None;
static mut IMAGE_BUFFER: Option<ImageBuffer> = None;
static mut MAPPING_RESULT: Option<anyhow::Result<()>> = None;

mod hook;
mod image_info;
mod signature;
mod uefi_core;
mod utils;
mod wdef;
mod winload;

fn initialize_output() -> uefi::Result<()> {
    let mut system_table = system_table();
    let stdout = system_table.stdout();

    let output_mode = stdout.modes().reduce(|acc, val| {
        if val.columns() * val.rows() < acc.columns() * acc.rows() {
            acc
        } else {
            val
        }
    });

    if let Some(output_mode) = output_mode {
        stdout.set_mode(output_mode)?;
    } else {
        /* Keep the current output mode as a fallback */
    }

    stdout.set_color(Color::White, Color::Blue)?;
    stdout.clear()?;

    Ok(())
}

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    let _exec_guard = enter_execution_context(ExecutionContext::UEFI);
    uefi_core::initialize(&system_table);

    if let Err(err) = real_main(handle, &mut system_table) {
        log::error!("{}", obfstr!("Valthrun bootstrap error"));
        log::error!("{:#}", err);
        press_enter_to_continue();

        Status::LOAD_ERROR
    } else {
        Status::SUCCESS
    }
}

fn real_main(handle: Handle, system_table: &mut SystemTable<Boot>) -> anyhow::Result<()> {
    initialize_output()
        .map_err(|err| anyhow!("{}: {:#?}", obfstr!("Failed to initialize output"), err))?;

    let bs = system_table.boot_services();
    let windows_bootmgr = find_windows_bootmgr(handle, bs)?
        .with_context(|| obfstr!("Could not find Windows boot manager").to_string())?;

    log::debug!(
        "{} {}",
        obfstr!("Windows boot manager located at"),
        *&windows_bootmgr
            .to_string(bs, DisplayOnly(true), AllowShortcuts(false))
            .map_err(|err| anyhow!("{:#}", err))?
            .ok_or_else(|| anyhow!("{}", obfstr!("expected the path to be non empty")))?
    );

    let bootmgr_handle = bs
        .load_image(
            handle,
            LoadImageSource::FromDevicePath {
                device_path: &windows_bootmgr,
                from_boot_manager: true,
            },
        )
        .map_err(|err| {
            anyhow!(
                "{}: {}",
                obfstr!("failed to load Windows boot manager"),
                err
            )
        })?;

    let bootmgr_image = ImageInfo::from_handle(bootmgr_handle.clone())?;
    setup_hooks_bootmgr(bootmgr_image)?;

    log::info!("Invoking bootmgr");
    if let Err(err) = { bs.start_image(bootmgr_handle) } {
        if let Err(err) = bs.unload_image(bootmgr_handle) {
            log::warn!(
                "{}: {:#}",
                obfstr!("Failed to unload Windows bootmgr image"),
                err
            );
        }

        anyhow::bail!(
            "{}: {}",
            obfstr!("failed to invoke Windows boot manager"),
            err
        )
    }

    log::error!(
        "{}",
        obfstr!("The Windows boot manager exited unexpectedly.")
    );
    Ok(())
}

fn get_device_name_from_variable(device_path_raw: &[u8]) -> Option<String> {
    let variable_keys = system_table().runtime_services().variable_keys().map_err(|e| {
        log::warn!("{}: {:?}", obfstr!("Failed to get variable keys"), e);
    }).ok()?;

    variable_keys.iter().find_map(|variable_key| {
        // Check if Boot#### variable
        let cstr_name = variable_key.name().ok().filter(|cstr| {
            cstr.to_string().starts_with(obfstr!("Boot"))
                && cstr.to_string().len() == 8
                && variable_key.vendor == uefi_raw::table::runtime::VariableVendor::GLOBAL_VARIABLE
        })?;

        // Get variable contents
        let (data, _) = system_table().runtime_services().get_variable_boxed(&cstr_name, &variable_key.vendor).ok()?;

        // EFI_LOAD_OPTION
        // Attributes(u32): 4 + FilePathListLenght(u16): 2
        let description_start = 6;

        let file_path_list_length = {
            let len = u16::from_le_bytes([data[4], data[5]]) as usize;
            (len != 0).then_some(len)?
        };

        let description = String::from_utf16(
            &data[description_start..]
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .take_while(|&u| u != 0)
                .collect::<Vec<u16>>()
        ).ok()?;
        

        // Attributes + FilePathListLength + Char16 chars * 2 bytes + null terminator (2 bytes)
        let file_path_list_start = description_start + (description.len() * 2) + 2;

        data.len().checked_sub(file_path_list_start).and_then(|_| {
            let file_path_list = &data[file_path_list_start..file_path_list_start + file_path_list_length];

            let mut nodes = Vec::new();
            let mut start = 0;

            let node_entire_end: [u8; 4] = [0x7f, 0xff, 0x04, 0x00];

            // Split the nodes
            for i in 0..(file_path_list.len() - 4 + 1) {
                if &file_path_list[i..i + 4] == &node_entire_end {
                    if start < i {
                        nodes.push(&file_path_list[start..i]);
                    }
                    start = i + 4;
                }
            }

            // Check if device_path starts with node
            nodes.iter().find_map(|node|
                device_path_raw.starts_with(node).then(|| description.clone())
            )
        })
    })
}

fn find_windows_bootmgr(
    imnage_handle: Handle,
    boot_services: &BootServices,
) -> anyhow::Result<Option<Box<DevicePath>>> {
    let file_systems = boot_services
        .locate_handle_buffer(SearchType::ByProtocol(&SimpleFileSystem::GUID))
        .map_err(|err| anyhow!("{}: {:#}", obfstr!("locating simple fs"), err))?;

    let mut found_devices = Vec::new();
    let windows_bootmgr_path = CStr16::from_u16_with_nul(WINDOWS_BOOTMGR_PATH).unwrap();

    for handle in file_systems.iter() {
        let device_path = boot_services
            .open_protocol_exclusive::<DevicePath>(*handle)
            .map_err(|err| anyhow!("{}: {:#}", obfstr!("open device path"), err))?;

        let file_system = unsafe {
            boot_services.open_protocol::<SimpleFileSystem>(
                OpenProtocolParams {
                    handle: handle.clone(),
                    agent: imnage_handle,
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
        };
        let file_system = match file_system {
            Ok(fs) => fs,
            Err(err) => {
                log::warn!(
                    "{} 0x{:X}: {}",
                    obfstr!("Failed to open simple fs handle"),
                    handle.as_ptr() as u64,
                    err
                );
                continue;
            }
        };
        let file_system = file_system
            .get_mut()
            .expect(obfstr!("the file system to be present"));

        let mut volume = match file_system.open_volume() {
            Ok(volume) => volume,
            Err(err) => {
                log::warn!(
                    "{} 0x{:X}: {:#?}",
                    obfstr!("Failed to open volume for simple fs handle"),
                    handle.as_ptr() as u64,
                    err
                );
                continue;
            }
        };
        
        let win_handle = volume.open(
            &windows_bootmgr_path,
            FileMode::Read,
            FileAttribute::READ_ONLY,
        );

        if win_handle.is_ok() {
            /* Windows boot manager has been found */
            let device_path_raw: &[u8] = device_path.as_bytes();
            let device_name = get_device_name_from_variable(device_path_raw)
                .unwrap_or_else(|| "unknown".to_string());

            found_devices.push((device_path, device_name));
        }
    }

    if !found_devices.is_empty() {
        let device_index = if found_devices.len() == 1 { 0 } else {
            show_select(found_devices.iter().map(|(_, name)| name.clone()).collect())
        };
        let device_path = &found_devices[device_index].0;
        let device_path = device_path
            .get()
            .expect(obfstr!("device path to be present"))
            .to_boxed();
    
        let mut buffer = Vec::new();
    
        let file_device_path = device_path.node_iter().fold(
            build::DevicePathBuilder::with_vec(&mut buffer),
            |acc, entry| acc.push(&entry).unwrap(),
        );
    
        let file_device_path = file_device_path
            .push(&build::media::FilePath {
                path_name: &windows_bootmgr_path,
            })
            .unwrap()
            .finalize()
            .unwrap();
    
        return Ok(Some(file_device_path.to_boxed()));
    }

    Ok(None)
}

unsafe extern "efiapi" fn hooked_exit_boot_services(
    image_handle: uefi_raw::Handle,
    map_key: usize,
) -> Status {
    let _exec_guard = enter_execution_context(ExecutionContext::UEFI);
    let original_fn = ORIGINAL_EXIT_BOOT_SERVICES.take().expect(obfstr!(
        "the original ExitBootServices callback to be saved"
    ));
    set_exit_boot_services(original_fn);

    fn finish_setup() -> anyhow::Result<()> {
        if unsafe { WINLOAD_IMAGE.is_none() } {
            anyhow::bail!(
                "{} has never been called.",
                obfstr!("ImgArchStartBootApplication")
            );
        }

        unsafe { MAPPING_RESULT.take() }
            .ok_or_else(|| anyhow!("{}", obfstr!("Mapping callback has never been called")))??;

        let image_buffer = unsafe { IMAGE_BUFFER.as_ref() }.with_context(|| {
            obfstr!("Never allocated the target images image buffer").to_string()
        })?;

        log::info!("ExitBootServices has been called.");
        log::info!("Mapped driver at {:X}", image_buffer.address as usize);

        Ok(())
    }

    if let Err(err) = finish_setup() {
        log::error!("Failed to map the Valthrun driver!");
        log::error!("{:#}", err);
        press_enter_to_continue();
    } else {
        log::info!("{}", obfstr!("Valthrun driver successfully mapped."));
        press_enter_to_continue();
        log::info!("Booting Windows...");
    }

    /*
     * Winload does not exists any more.
     * Technically it exists 'till the original exit boot services function
     * returns success else we're trying the next option (max 4).
     */
    winload::finalize();

    (original_fn)(image_handle, map_key)
}

fn setup_hooks_bootmgr(image: ImageInfo) -> anyhow::Result<()> {
    let func_address = image.resolve_signature(&Signature::pattern("ImgArchStartBootApplication", "48 8B C4 48 89 58 ? 44 89 40 ? 48 89 50 ? 48 89 48 ? 55 56 57 41 54 41 55 41 56 41 57 48 8D 68 ? 48 81 EC C0 00 00 00"))?;

    unsafe {
        HOOK_IMG_ARCH_START_BOOT_APPLICATION
            .initialize_trampoline(ImgArchStartBootApplication::from_ptr_usize(func_address));
        HOOK_IMG_ARCH_START_BOOT_APPLICATION.enable(hooked_img_arch_start_boot_application);
    };

    Ok(())
}

fn setup_hooks_winload(image: ImageInfo) -> anyhow::Result<()> {
    winload::initialize(&image)?;

    let bl_img_allocate_image_buffer = [
        Signature::relative_address(
            obfstr!("BlImgAllocateImageBuffer (2600.1252)"),
            obfstr!("E8 ? ? ? ? 4C 8B 75 D8 8B D8 85 C0 0F"),
            0x01,
            0x05,
        ),
        /* Windows 11 */
        Signature::relative_address(
            obfstr!("BlImgAllocateImageBuffer (11)"),
            obfstr!("E8 ? ? ? ? 4C 8B 7D 50 8B"),
            0x01,
            0x05,
        ),
        /* Windows 10 19045.4046 (efi) */
        Signature::relative_address(
            obfstr!("BlImgAllocateImageBuffer (19045.4046/efi)"),
            obfstr!("E8 ? ? ? ? 4C 8B 6D 60"),
            0x01,
            0x05,
        ),
        /* Windows 10 19045.4046 (exe) */
        Signature::relative_address(
            obfstr!("BlImgAllocateImageBuffer (19045.4046/exe)"),
            obfstr!("E8 ? ? ? ? 4C 8B 65 50 8B"),
            0x01,
            0x05,
        ),
    ]
    .into_iter()
    .find_map(|sig| image.resolve_signature(&sig).ok())
    .with_context(|| obfstr!("Failed to locate BlImgAllocateImageBuffer signature").to_string())?;

    let osl_fwp_kernel_setup_phase1 = image.resolve_signature(&Signature::pattern(
        obfstr!("OslFwpKernelSetupPhase1"),
        obfstr!("48 89 4C 24 08 55 53 56 57 41 54 41 55 41 56 41 57 48 8D"),
    ))?;

    unsafe {
        WINLOAD_IMAGE = Some(image);

        HOOK_BL_IMG_ALLOCATE_IMAGE_BUFFER.initialize_trampoline(
            BlImgAllocateImageBuffer::from_ptr_usize(bl_img_allocate_image_buffer),
        );
        HOOK_OSL_FWP_KERNEL_SETUP_PHASE1.initialize_trampoline(
            OslFwpKernelSetupPhase1::from_ptr_usize(osl_fwp_kernel_setup_phase1),
        );

        HOOK_BL_IMG_ALLOCATE_IMAGE_BUFFER.enable(hooked_bl_img_allocate_image_buffer);
        HOOK_OSL_FWP_KERNEL_SETUP_PHASE1.enable(hooked_osl_fwp_kernel_setup_phase1);

        ORIGINAL_EXIT_BOOT_SERVICES = Some(set_exit_boot_services(hooked_exit_boot_services));
    }

    Ok(())
}

trait LoaderParameterBlockEx {
    fn find_module(&self, name: &str) -> anyhow::Result<Option<&KLDR_DATA_TABLE_ENTRY>>;
}

impl LoaderParameterBlockEx for LoaderParameterBlock {
    fn find_module(&self, name: &str) -> anyhow::Result<Option<&KLDR_DATA_TABLE_ENTRY>> {
        let mut current_entry = self.LoadOrderListHead.Flink;

        while current_entry as *const _ != &self.LoadOrderListHead {
            let entry = unsafe {
                current_entry
                    .cast::<KLDR_DATA_TABLE_ENTRY>()
                    .as_ref()
                    .with_context(|| obfstr!("flink not to be null").to_string())?
            };
            current_entry = unsafe { current_entry.as_ref() }
                .with_context(|| obfstr!("flink not to be null").to_string())?
                .Flink;

            let base_image_name = unsafe {
                slice::from_raw_parts(
                    entry.BaseImageName.Buffer,
                    (entry.BaseImageName.Length / 2) as usize,
                )
            };

            let image_name = String::from_utf16_lossy(base_image_name);
            if image_name == name {
                return Ok(Some(entry));
            }
        }

        return Ok(None);
    }
}

fn handle_osl_lpb(lpb: *mut LoaderParameterBlock) -> anyhow::Result<()> {
    log::debug!("handle_osl_lpb called with {:X}", lpb as u64);

    let lpb = unsafe { &*lpb };
    let hijacked_driver = lpb
        .find_module("acpiex.sys")?
        .with_context(|| obfstr!("could not find the windows kernel module").to_string())?;

    {
        let full_image_name = unsafe {
            slice::from_raw_parts(
                hijacked_driver.BaseImageName.Buffer as *mut u16,
                (hijacked_driver.BaseImageName.Length / 2) as usize,
            )
        };

        let hijacked_image_base = hijacked_driver.ImageBase;
        log::debug!(
            "Hijacked driver at {:X} ({})",
            hijacked_image_base as u64,
            String::from_utf16_lossy(full_image_name)
        );
    }

    let hijacked_driver_memory = unsafe {
        slice::from_raw_parts_mut(
            hijacked_driver.ImageBase as *mut u8,
            hijacked_driver.SizeOfImage as usize,
        )
    };

    let image_buffer = unsafe { IMAGE_BUFFER.as_mut() }
        .with_context(|| obfstr!("Expected to have allocated memory").to_string())?;
    let base_address = image_buffer.address as u64;
    map_custom_driver(
        hijacked_driver_memory,
        &TARGET_DRIVER,
        image_buffer.as_slice_mut(),
        base_address,
    )
    .with_context(|| obfstr!("mapping error").to_string())?;

    Ok(())
}

/*
 * Map the specially crafted / designed driver.
 * The driver needs to have the following properties:
 * 1. Do not rely on any external imports.
 *    Imports are not getting resolved.
 * 2. Do not rely on any sections other then .text and .data.
 *    All other sections will not be mapped, including the PE header.
 * 3. SEH is not available.
 */
fn map_custom_driver(
    hijacked_driver: &mut [u8],
    target_driver_file: &[u8],
    memory: &mut [u8],
    base_address: u64,
) -> anyhow::Result<()> {
    /* fill the memory with all zeros */
    memory.fill(0x00);

    let pe = PeFile::from_bytes(target_driver_file).map_err(Error::msg)?;

    /* map the sections */
    log::debug!("Mapping {} sections", pe.section_headers().as_slice().len());
    {
        for section in pe.section_headers() {
            let section_name = String::from_utf8_lossy(section.name_bytes()).to_string();
            let should_map = match section_name.as_str() {
                ".text" => true,
                ".data" => true,
                _ => true,
            };

            if !should_map {
                log::debug!(" Skipping {}", section_name);
                continue;
            }

            if (section.VirtualAddress + section.SizeOfRawData) as usize > memory.len() {
                anyhow::bail!("section {} is longer then the available memory (va: {:X}, size: {:X}, memory: {:X})", section_name, section.VirtualAddress, section.SizeOfRawData, memory.len());
            }

            if (section.PointerToRawData + section.SizeOfRawData) as usize
                > target_driver_file.len()
            {
                anyhow::bail!("section {} references invalid data (raw data: {:X}, size: {:X}, file length: {:X})", section_name, section.VirtualAddress, section.SizeOfRawData, target_driver_file.len());
            }

            let section_memory = &mut memory[section.VirtualAddress as usize
                ..(section.VirtualAddress + section.SizeOfRawData) as usize];
            let section_data = &target_driver_file[section.PointerToRawData as usize
                ..(section.PointerToRawData + section.SizeOfRawData) as usize];
            section_memory.copy_from_slice(section_data);

            log::debug!(" Mapped {}", section_name);
        }
    }

    /* Relocations */
    let relocs = pe.base_relocs().map_err(Error::msg)?;
    for reloc_block in relocs.iter_blocks() {
        for reloc in reloc_block.words() {
            match reloc_block.type_of(reloc) {
                0x00 => {
                    /* IMAGE_REL_BASED_ABSOLUTE */
                    continue;
                }
                0x0A => {
                    /* IMAGE_REL_BASED_DIR64 */
                    let rva = reloc_block.rva_of(reloc) as usize;
                    let value = u64::from_le_bytes(memory[rva..rva + 8].try_into().unwrap());
                    let image_base = match pe.optional_header() {
                        Wrap::T32(header) => header.ImageBase as u64,
                        Wrap::T64(header) => header.ImageBase as u64,
                    };

                    if value < image_base {
                        /*
                         * This can occurr because of two reasons:
                         * 1. The section has not been mapped
                         * 2. The relocation is invalid
                         */
                        continue;
                    }

                    let new_address = base_address + value - image_base;
                    memory[rva..rva + 8].copy_from_slice(&new_address.to_le_bytes());
                }
                reloc_type => {
                    anyhow::bail!("{} {:X}", obfstr!("Unsupported reloc type"), reloc_type)
                }
            }
        }
    }

    /* Hijack the target driver and save original instructions to the mapped driver */
    {
        let rva_hijacked_entry_point = {
            let hijacked_pe = PeView::from_bytes(hijacked_driver)
                .map_err(Error::msg)
                .with_context(|| obfstr!("hijacked driver").to_string())?;

            match hijacked_pe.optional_header() {
                Wrap::T32(header) => header.AddressOfEntryPoint,
                Wrap::T64(header) => header.AddressOfEntryPoint,
            }
        } as usize;

        let rva_driver_entry_point = match pe.optional_header() {
            Wrap::T32(header) => header.AddressOfEntryPoint,
            Wrap::T64(header) => header.AddressOfEntryPoint,
        } as usize;

        let exports = pe.exports().map_err(Error::msg)?.by().map_err(Error::msg)?;
        let rva_original_entry_bytes = exports
            .name_linear("_ENTRY_BYTES")
            .ok()
            .with_context(|| obfstr!("Could not find _ENTRY_BYTES export").to_string())?
            .symbol()
            .with_context(|| obfstr!("Expected the _ENTRY_BYTES to be a symbol").to_string())?
            as usize;

        /* Store the original entry point data */
        const DRIVER_ENTRYPOINT_BUFFER_SIZE: usize = 0x20;
        memory[rva_original_entry_bytes..rva_original_entry_bytes + DRIVER_ENTRYPOINT_BUFFER_SIZE]
            .copy_from_slice(
                &hijacked_driver[rva_hijacked_entry_point
                    ..rva_hijacked_entry_point + DRIVER_ENTRYPOINT_BUFFER_SIZE],
            );

        let mut instructions = Vec::<u8>::with_capacity(DRIVER_ENTRYPOINT_BUFFER_SIZE);

        {
            /* lea r8, [rip-7] */
            instructions.extend(&[0x4C, 0x8D, 0x05, 0xF9, 0xFF, 0xFF, 0xFF]);

            // /* nop */
            // instructions.push(0xCC);

            /* jmp DWORD PTR ds:0x0 */
            instructions.extend(&[0xFF, 0x25, 0x00, 0x00, 0x00, 0x00]);

            /* the target address */
            instructions.extend(&(base_address + rva_driver_entry_point as u64).to_le_bytes());
        }

        if instructions.len() > DRIVER_ENTRYPOINT_BUFFER_SIZE {
            anyhow::bail!(
                "{}",
                obfstr!("shellcode is longer then saved entry point bytes")
            )
        }

        hijacked_driver[rva_hijacked_entry_point..rva_hijacked_entry_point + instructions.len()]
            .copy_from_slice(&instructions);

        log::debug!(
            "Hijacked driver entry point at {:X}. Original bytes at {:X}",
            hijacked_driver.as_ptr() as usize + rva_hijacked_entry_point,
            memory.as_ptr() as usize + rva_original_entry_bytes
        );
    }

    Ok(())
}
