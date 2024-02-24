use alloc::{
    format,
    vec::Vec,
};

use uefi::CStr16;

use crate::{
    context::{
        current_execution_context,
        enter_execution_context,
        system_table,
        ExecutionContext,
    },
    winload::{
        self,
        WinloadContext,
    },
};

pub struct KernelLogger;

impl log::Log for KernelLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if cfg!(debug_assertions) {
            true
        } else {
            metadata.level() <= log::Level::Debug
        }
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        match current_execution_context() {
            ExecutionContext::UNKNOWN | ExecutionContext::UEFI | ExecutionContext::WINBOOTMGR => {
                /* we're good to go :) */
            }
            ExecutionContext::WINLOAD => {
                let _guard = winload::enter_context(WinloadContext::FirmwareExecutionContext);
                let _context = enter_execution_context(ExecutionContext::UEFI);

                self.log(record);
                return;
            }
        }

        let level_prefix = match record.level() {
            log::Level::Trace => "T",
            log::Level::Debug => "D",
            log::Level::Info => "I",
            log::Level::Warn => "W",
            log::Level::Error => "E",
        };
        let payload = if cfg!(debug_assertions) {
            format!("[{}] {}\r\n\0", level_prefix, record.args())
        } else {
            format!("[{}] {}\r\n\0", level_prefix, record.args())
        };

        let payload = payload.encode_utf16().collect::<Vec<_>>();
        let _ = system_table()
            .stdout()
            .output_string(&unsafe { CStr16::from_u16_with_nul_unchecked(&payload) });
    }

    fn flush(&self) {}
}

pub static APP_LOGGER: KernelLogger = KernelLogger;
