use core::{
    ffi::c_void,
    sync::atomic::{
        AtomicPtr,
        Ordering,
    },
};

use uefi::table::{
    Boot,
    SystemTable,
};

use crate::uefi_core::{
    current_execution_context,
    ExecutionContext,
};

static SYSTEM_TABLE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

pub fn setup_system_table(system_table: &SystemTable<Boot>) {
    SYSTEM_TABLE.store(
        unsafe { core::mem::transmute_copy(system_table) },
        Ordering::Release,
    );
}

#[must_use]
pub fn system_table_opt() -> Option<SystemTable<Boot>> {
    let ptr = SYSTEM_TABLE.load(Ordering::Acquire);
    // Safety: the `SYSTEM_TABLE` pointer either be null or a valid system
    // table.
    //
    // Null is the initial value, as well as the value set when exiting boot
    // services. Otherwise, the value is set by the call to `init`, which
    // requires a valid system table reference as input.
    unsafe { SystemTable::from_ptr(ptr) }
}

/// Obtains a pointer to the system table.
///
/// This is meant to be used by higher-level libraries,
/// which want a convenient way to access the system table singleton.
///
/// `init` must have been called first by the UEFI app.
///
/// The returned pointer is only valid until boot services are exited.
#[must_use]
pub fn system_table() -> SystemTable<Boot> {
    assert!(matches!(
        current_execution_context(),
        ExecutionContext::UEFI | ExecutionContext::UNKNOWN | ExecutionContext::WINBOOTMGR
    ));
    system_table_opt().expect("The system table handle is not available")
}
