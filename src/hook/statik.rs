use core::marker::PhantomData;

use super::{
    Function,
    Hook,
    TrampolineHook,
};

pub struct StaticHook<T: Function, H: Hook<T>> {
    inner: Option<H>,
    _dummy: PhantomData<T>,
}

impl<T: Function, H: Hook<T>> StaticHook<T, H> {
    pub const fn new() -> Self {
        Self {
            inner: None,
            _dummy: PhantomData {},
        }
    }

    pub fn initialize(&mut self, hook: H) {
        assert!(self.inner.is_none());
        self.inner = Some(hook);
    }

    pub fn initialized(&self) -> bool {
        self.inner.is_some()
    }

    pub fn target(&self) -> Option<T> {
        self.inner.as_ref().map(Hook::target)
    }

    pub fn is_active(&self) -> bool {
        self.inner.as_ref().map(Hook::is_active).unwrap_or(false)
    }

    pub unsafe fn enable(&mut self, target: T) -> bool {
        match &mut self.inner {
            Some(inner) => inner.enable(target),
            None => false,
        }
    }

    pub unsafe fn disable(&mut self) {
        if let Some(inner) = &mut self.inner {
            inner.disable();
        }
    }
}

impl<T: Function> StaticHook<T, TrampolineHook<T>> {
    pub fn initialize_trampoline(&mut self, target: T) {
        self.initialize(TrampolineHook::create(target))
    }
}
