/// Trait representing a function that can be used as a target or detour for
/// detouring.
pub unsafe trait Function: Sized + Copy + Sync + 'static {
    /// The argument types as a tuple.
    type Arguments;

    /// The return type.
    type Output;

    /// Constructs a `Function` from an untyped pointer.
    unsafe fn from_ptr(ptr: *const ()) -> Self;

    /// Constructs a `Function` from an untyped usize pointer.
    unsafe fn from_ptr_usize(ptr: usize) -> Self {
        Self::from_ptr(ptr as *const ())
    }

    /// Returns an untyped pointer for this function.
    fn to_ptr(&self) -> *const ();

    fn to_ptr_usize(&self) -> usize {
        self.to_ptr() as usize
    }
}

macro_rules! impl_function {
    (@recurse () ($($nm:ident : $ty:ident),*)) => {
      impl_function!(@impl_all ($($nm : $ty),*));
    };
    (@recurse
        ($hd_nm:ident : $hd_ty:ident $(, $tl_nm:ident : $tl_ty:ident)*)
        ($($nm:ident : $ty:ident),*)) => {
      impl_function!(@impl_all ($($nm : $ty),*));
      impl_function!(@recurse ($($tl_nm : $tl_ty),*) ($($nm : $ty,)* $hd_nm : $hd_ty));
    };

    (@impl_all ($($nm:ident : $ty:ident),*)) => {
      impl_function!(@impl_pair ($($nm : $ty),*) (                  fn($($ty),*) -> Ret));
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "cdecl"    fn($($ty),*) -> Ret));
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "efiapi"   fn($($ty),*) -> Ret));
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "stdcall"  fn($($ty),*) -> Ret));
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "fastcall" fn($($ty),*) -> Ret));
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "win64"    fn($($ty),*) -> Ret));
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "C"        fn($($ty),*) -> Ret));
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "system"   fn($($ty),*) -> Ret));

      #[cfg_attr(docsrs, doc(cfg(feature = "thiscall-abi")))]
      impl_function!(@impl_pair ($($nm : $ty),*) (extern "thiscall" fn($($ty),*) -> Ret));
    };

    (@impl_pair ($($nm:ident : $ty:ident),*) ($($fn_t:tt)*)) => {
      impl_function!(@impl_fun ($($nm : $ty),*) ($($fn_t)*) (unsafe $($fn_t)*));
    };

    (@impl_fun ($($nm:ident : $ty:ident),*) ($safe_type:ty) ($unsafe_type:ty)) => {
      impl_function!(@impl_core ($($nm : $ty),*) ($safe_type));
      impl_function!(@impl_core ($($nm : $ty),*) ($unsafe_type));
    };

    (@impl_core ($($nm:ident : $ty:ident),*) ($fn_type:ty)) => {
      unsafe impl<Ret: 'static, $($ty: 'static),*> Function for $fn_type {
        type Arguments = ($($ty,)*);
        type Output = Ret;

        unsafe fn from_ptr(ptr: *const ()) -> Self {
          ::core::mem::transmute(ptr)
        }

        fn to_ptr(&self) -> *const () {
          *self as *const ()
        }
      }
    };

    ($($nm:ident : $ty:ident),*) => {
      impl_function!(@recurse ($($nm : $ty),*) ());
    };
  }

impl_function! {
  __arg_0:  A, __arg_1:  B, __arg_2:  C, __arg_3:  D, __arg_4:  E, __arg_5:  F, __arg_6:  G,
  __arg_7:  H, __arg_8:  I, __arg_9:  J, __arg_10: K, __arg_11: L, __arg_12: M, __arg_13: N
}
