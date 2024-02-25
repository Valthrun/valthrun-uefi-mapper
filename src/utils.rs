use core::mem;

use uefi::proto::console::text::{
    Key,
    ScanCode,
};

use crate::{
    system_table,
    FnExitBootServices,
};

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
