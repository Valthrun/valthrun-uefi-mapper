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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum ExecutionContext {
    UNKNOWN,
    UEFI,
    WINBOOTMGR,
    WINLOAD,
}

static mut CURRENT_CONTEXT: ExecutionContext = ExecutionContext::UNKNOWN;

pub fn current_execution_context() -> ExecutionContext {
    unsafe { CURRENT_CONTEXT }
}

#[must_use]
pub fn enter_execution_context(context: ExecutionContext) -> ExecutionContextGuard {
    let guard = ExecutionContextGuard {
        previous: current_execution_context(),
    };
    unsafe { CURRENT_CONTEXT = context };
    guard
}

pub struct ExecutionContextGuard {
    previous: ExecutionContext,
}

impl Drop for ExecutionContextGuard {
    fn drop(&mut self) {
        unsafe {
            CURRENT_CONTEXT = self.previous;
        }
    }
}

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
