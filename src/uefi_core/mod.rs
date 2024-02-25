use uefi::table::{
    Boot,
    SystemTable,
};

mod allocator;
mod context;
mod logger;
mod panic;
mod system_table;

pub use context::*;
pub use system_table::*;

use self::logger::APP_LOGGER;

pub fn initialize(system_table: &SystemTable<Boot>) {
    system_table::setup_system_table(&system_table);
    unsafe { uefi::allocator::init(system_table.boot_services()) };

    let _ = log::set_logger(&APP_LOGGER);
    log::set_max_level(log::STATIC_MAX_LEVEL);
}
