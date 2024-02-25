use alloc::format;
use core::{
    arch::asm,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

use uefi::CStr16;

use super::{
    current_execution_context,
    enter_execution_context,
    system_table_opt,
    ExecutionContext,
};
use crate::winload::{
    self,
    WinloadContext,
};

static PANIC_COUNT: AtomicUsize = AtomicUsize::new(0);

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if PANIC_COUNT.fetch_add(1, Ordering::Relaxed) > 0 {
        /* a panic occurred within a panic... */
        loop {
            unsafe { asm!("int 3") };
        }
    }

    panic_handler_impl(info)
}

fn panic_handler_impl(info: &core::panic::PanicInfo) -> ! {
    if current_execution_context() == ExecutionContext::WINLOAD {
        let _guard = winload::enter_context(WinloadContext::FirmwareExecutionContext);
        let _context = enter_execution_context(ExecutionContext::UEFI);
        panic_handler_impl(info);
    }

    let mut system_table = match system_table_opt() {
        Some(st) => st,
        None => loop {
            unsafe { asm!("int 3") };
        },
    };

    /* Write a panic message without any allocations in case something within the allocator paniced */
    {
        let buffer = obfstr::wide!("PANIC OCCURRED!\n\r\0");
        let message = unsafe { CStr16::from_u16_with_nul(buffer).unwrap_unchecked() };
        let _ = system_table.stdout().output_string(message);
    }

    let message = format!("{}", info);
    for line in message.lines() {
        log::error!("{}", line);
    }

    system_table.boot_services().stall(10_000_000);
    loop {
        unsafe { asm!("int 3") };
    }
}
