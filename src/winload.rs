use core::mem;

use obfstr::obfstr;

use crate::{
    signature::Signature,
    ImageInfo,
};

type FnBlpArchSwitchContext = unsafe extern "efiapi" fn(ContextId: u32);

static mut CURRENT_EXECUTION_CONTEXT: Option<*mut u32> = None;
static mut BLP_ARCH_SWITCH_CONTEXT: Option<FnBlpArchSwitchContext> = None;

pub fn initialize(image: &ImageInfo) -> anyhow::Result<()> {
    let current_execution_context = image.resolve_signature(&Signature::relative_address(
        obfstr!("CurrentExecutionContext"),
        obfstr!("48 8B 05 ? ? ? ? 4C 8D 7D D0"),
        0x03,
        0x07,
    ))? as *mut u32;

    let blp_arch_switch_context = image.resolve_signature(&Signature::relative_address(
        obfstr!("BlpArchSwitchContext"),
        obfstr!("E8 ? ? ? ? 48 8B 43 08 49"),
        0x01,
        0x05,
    ))?;

    unsafe {
        CURRENT_EXECUTION_CONTEXT = Some(current_execution_context);
        BLP_ARCH_SWITCH_CONTEXT = Some(mem::transmute(blp_arch_switch_context));
    }

    Ok(())
}

pub fn finalize() {
    unsafe {
        CURRENT_EXECUTION_CONTEXT = None;
        BLP_ARCH_SWITCH_CONTEXT = None;
    }
}

pub struct WinloadContextGuard {
    original_context: Option<u32>,
}

impl WinloadContextGuard {
    fn enter_context(target_context: u32) -> Self {
        let current_context = unsafe {
            CURRENT_EXECUTION_CONTEXT
                .clone()
                .map(|level| level.read_volatile())
                .unwrap_or(target_context)
        };

        if current_context == target_context {
            Self {
                original_context: None,
            }
        } else if let Some(switch_context) = unsafe { BLP_ARCH_SWITCH_CONTEXT } {
            unsafe {
                switch_context(target_context);
            }

            Self {
                original_context: Some(current_context),
            }
        } else {
            /* CURRENT_EXECUTION_CONTEXT is initialized, but BLP_ARCH_SWITCH_CONTEXT isnt... */
            Self {
                original_context: None,
            }
        }
    }
}

impl Drop for WinloadContextGuard {
    fn drop(&mut self) {
        if let Some(original) = self.original_context.take() {
            if let Some(switch_context) = unsafe { BLP_ARCH_SWITCH_CONTEXT } {
                unsafe {
                    (switch_context)(original);
                }
            }
        }
    }
}

#[allow(dead_code)]
pub enum WinloadContext {
    /// Winload context with virtual address space.
    ApplicationExecutionContext,

    /// EFI context, required for all EFI functions
    /// (physical address space)
    FirmwareExecutionContext,
}

impl WinloadContext {
    fn context_id(&self) -> u32 {
        match self {
            Self::ApplicationExecutionContext => 0,
            Self::FirmwareExecutionContext => 1,
        }
    }
}

pub fn enter_context(target_context: WinloadContext) -> WinloadContextGuard {
    WinloadContextGuard::enter_context(target_context.context_id())
}

#[allow(dead_code)]
pub fn assert_context(target_context: WinloadContext) {
    if let Some(context) = unsafe { CURRENT_EXECUTION_CONTEXT.clone() } {
        let current_context = unsafe { context.read_volatile() };
        if current_context != target_context.context_id() {
            let _guard = self::enter_context(WinloadContext::FirmwareExecutionContext);
            panic!(
                "{} {:X} but it is {:X}",
                obfstr!("Expected the winload context to be"),
                target_context.context_id(),
                current_context
            );
        }
    }
}
