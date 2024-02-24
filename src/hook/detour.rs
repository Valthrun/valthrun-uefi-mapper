use core::marker::PhantomData;

use super::{
    Function,
    Hook,
};

#[rustfmt::skip]
const HOOK_SHELLCODE: [u8; 14] = [
    /* jmp    DWORD PTR ds:0x0 */
    0xFF, 0x25, 0x00, 0x00, 0x00, 0x00, 
    
    /* target address */
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub struct TrampolineHook<T> {
    hooked: bool,

    target: T,
    bytes_original: [u8; HOOK_SHELLCODE.len()],

    _dummy: PhantomData<T>,
}

unsafe impl<T> Send for TrampolineHook<T> {}
unsafe impl<T> Sync for TrampolineHook<T> {}

impl<T: Function> TrampolineHook<T> {
    pub fn create(target: T) -> Self {
        let original = [0u8; HOOK_SHELLCODE.len()];
        Self {
            hooked: false,
            target,
            bytes_original: original,

            _dummy: Default::default(),
        }
    }

    unsafe fn bytes_target(&self) -> &'static mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self.target.to_ptr() as *mut u8, HOOK_SHELLCODE.len())
        }
    }
}

impl<T: Function> Hook<T> for TrampolineHook<T> {
    fn is_active(&self) -> bool {
        self.hooked
    }

    fn target(&self) -> T {
        self.target
    }

    unsafe fn enable(&mut self, target: T) -> bool {
        let bytes_target = self.bytes_target();
        if !self.hooked {
            self.hooked = true;

            self.bytes_original.copy_from_slice(&bytes_target);
            bytes_target.copy_from_slice(&HOOK_SHELLCODE);
        }

        bytes_target[6..].copy_from_slice(&target.to_ptr_usize().to_le_bytes());
        true
    }

    unsafe fn disable(&mut self) {
        if !self.hooked {
            return;
        }

        self.hooked = false;
        let bytes_target = self.bytes_target();
        bytes_target.copy_from_slice(&self.bytes_original);
    }
}
