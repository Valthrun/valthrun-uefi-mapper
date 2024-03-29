#![allow(unused)]
//! Windows kernel and loader definitions

#[repr(C)]
#[allow(non_snake_case)]
pub struct ListEntry {
    pub Flink: *mut ListEntry,
    pub Blink: *mut ListEntry,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct LoaderParameterBlock {
    pub OsMajorVersion: u32,
    pub OsMinorVersion: u32,
    pub Size: u32,
    pub OsLoaderSecurityVersion: u32,
    pub LoadOrderListHead: ListEntry,
}

#[repr(C)]
#[allow(non_snake_case, non_camel_case_types)]
pub struct KLDR_DATA_TABLE_ENTRY {
    pub InLoadOrderLinks: ListEntry,
    pub ExceptionTable: *const (),
    pub ExceptionTableSize: u32,
    pub GpValue: *const (),
    pub NonPagedDebugInfo: *const (),
    pub ImageBase: *const (),
    pub EntryPoint: *const (),
    pub SizeOfImage: u32,
    pub FullImageName: UNICODE_STRING,
    pub BaseImageName: UNICODE_STRING,
}

#[repr(C)]
#[allow(non_snake_case, non_camel_case_types)]
pub struct UNICODE_STRING {
    pub Length: u16,
    pub MaximumLength: u16,
    pub Buffer: *mut u16,
}

#[allow(non_camel_case_types)]
pub type NT_STATUS = i32;

pub type ImgArchStartBootApplication = extern "efiapi" fn(
    app_entry: *const (),
    image_base: *mut u8,
    image_size: u32,
    boot_option: u8,
    return_arguments: *mut (),
) -> u32;

pub type BlImgAllocateImageBuffer = extern "efiapi" fn(
    image_buffer: *mut *mut u8,
    image_size: usize,
    memory_type: u32,
    attributes: u32,
    unused: u64,
    flags: u32,
) -> NT_STATUS;

pub type OslFwpKernelSetupPhase1 = extern "efiapi" fn(lpb: *mut LoaderParameterBlock) -> u32;

pub const BL_MEMORY_TYPE_KERNEL: u32 = 0xE0000012; /* ntoskrnl.exe, kdstub.dll, kdcom.dll, hal.dll, mcupdate.dll */
pub const BL_MEMORY_TYPE_DRIVER: u32 = 0xE0000013; /* all other normal drivers */
pub const BL_MEMORY_TYPE_KERNEL_SECURE: u32 = 0xE0000022;
pub const BL_MEMORY_ATTRIBUTE_RWX: u32 = 0x424000; /* Value taken from a dummy bl img allocate buffer call with driver type */
