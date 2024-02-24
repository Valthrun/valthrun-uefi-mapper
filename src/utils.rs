use uefi::proto::console::text::{
    Key,
    ScanCode,
};

use crate::system_table;

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
