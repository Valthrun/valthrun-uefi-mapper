use alloc::{
    string::String,
    vec::Vec,
};
use core::mem;

use obfstr::obfstr;
use uefi::proto::console::text::{
    Key,
    ScanCode,
};

use crate::{
    system_table,
    FnExitBootServices,
};

pub fn show_select(devices_name: Vec<String>) -> usize {
    system_table().stdout().enable_cursor(false).ok();

    let mut init = true;
    let mut current_index: usize = 0;

    let enter_key = uefi::Char16::try_from('\r').unwrap();

    while let Ok(event) = system_table().stdin().read_key() {
        if init {
            init = false;
        } else {
            let key = match event {
                Some(key) => key,
                None => continue,
            };

            match key {
                Key::Printable(ch) if ch == enter_key => return current_index,

                Key::Special(ScanCode::DOWN) if current_index < devices_name.len() - 1 => {
                    current_index += 1
                }

                Key::Special(ScanCode::UP) if current_index > 0 => current_index -= 1,

                _ => continue,
            }
        }

        system_table().stdout().clear().ok();

        log::info!(
            "{}",
            obfstr!("\r  Arrow Up/Down: Move cursor\r\n  Enter: Select\n")
        );
        log::info!("{}", obfstr!("\r  Select device:"));

        for (i, device_name) in devices_name.iter().enumerate() {
            log::info!(
                "{} {} {}: ({})",
                if i == current_index { "\r>" } else { "\r " },
                obfstr!("Device"),
                i + 1,
                device_name
            );
        }
    }
    current_index
}

pub fn press_enter_to_continue() {
    log::info!("Press F10 to continue");
    while let Ok(event) = system_table().stdin().read_key() {
        let key = match event {
            Some(key) => key,
            None => continue,
        };

        if matches!(key, Key::Special(ScanCode::FUNCTION_10)) {
            break;
        }
    }
}

pub fn set_exit_boot_services(target: FnExitBootServices) -> FnExitBootServices {
    let raw_bs = unsafe {
        mem::transmute_copy::<_, &mut uefi_raw::table::boot::BootServices>(
            &system_table().boot_services(),
        )
    };

    mem::replace(&mut raw_bs.exit_boot_services, target)
}

#[repr(C)] // guarantee 'bytes' comes after '_align'
pub struct AlignedAs<Align, Bytes: ?Sized> {
    pub _align: [Align; 0],
    pub bytes: Bytes,
}

macro_rules! include_bytes_align_as {
    ($align_ty:ty, $path:literal) => {{
        // const block expression to encapsulate the static
        use $crate::utils::AlignedAs;

        // this assignment is made possible by CoerceUnsized
        static ALIGNED: &AlignedAs<$align_ty, [u8]> = &AlignedAs {
            _align: [],
            bytes: *include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}
pub(crate) use include_bytes_align_as;
