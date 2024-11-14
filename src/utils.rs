use core::mem;

use uefi::proto::console::text::{
    Key,
    ScanCode,
};

use crate::{
    system_table,
    FnExitBootServices,
};

pub fn show_select(num_devices: usize) -> usize {
    log::info!("Select device to boot:");
    for index in 0..num_devices.min(12) {
        log::info!("F{}: Device {}", index + 1, index + 1);
    }

    loop {
        if let Ok(event) = system_table().stdin().read_key() {
            let key = match event {
                Some(key) => key,
                None => continue,
            };

            if let Key::Special(scancode) = key {
                let scan_value = match scancode {
                    ScanCode::FUNCTION_1 => 1,
                    ScanCode::FUNCTION_2 => 2,
                    ScanCode::FUNCTION_3 => 3,
                    ScanCode::FUNCTION_4 => 4,
                    ScanCode::FUNCTION_5 => 5,
                    ScanCode::FUNCTION_6 => 6,
                    ScanCode::FUNCTION_7 => 7,
                    ScanCode::FUNCTION_8 => 8,
                    ScanCode::FUNCTION_9 => 9,
                    ScanCode::FUNCTION_10 => 10,
                    ScanCode::FUNCTION_11 => 11,
                    ScanCode::FUNCTION_12 => 12,
                    _ => continue,
                };

                if scan_value >= 1 && scan_value <= num_devices {
                    return (scan_value - 1) as usize;
                }
            }
        }
    }
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
