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
