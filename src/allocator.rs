use core::alloc::GlobalAlloc;

use uefi::allocator::Allocator as UefiAllocator;

use crate::{
    context::{
        current_execution_context,
        ExecutionContext,
    },
    winload::{
        self,
        WinloadContext,
    },
};

struct WinUefiAllocator {
    inner: UefiAllocator,
}

unsafe impl GlobalAlloc for WinUefiAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if matches!(current_execution_context(), ExecutionContext::WINLOAD) {
            let _ = winload::enter_context(WinloadContext::FirmwareExecutionContext);
            self.inner.alloc(layout)
        } else {
            self.inner.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if matches!(current_execution_context(), ExecutionContext::WINLOAD) {
            let _ = winload::enter_context(WinloadContext::FirmwareExecutionContext);
            self.inner.dealloc(ptr, layout)
        } else {
            self.inner.dealloc(ptr, layout)
        }
    }
}

#[global_allocator]
static ALLOCATOR: WinUefiAllocator = WinUefiAllocator {
    inner: UefiAllocator {},
};
