pub trait Hook<T: Function> {
    fn is_active(&self) -> bool;
    fn target(&self) -> T;

    unsafe fn enable(&mut self, target: T) -> bool;
    unsafe fn disable(&mut self);
}

mod detour;
pub use detour::*;

mod statik;
pub use statik::*;

mod function;
pub use function::*;
