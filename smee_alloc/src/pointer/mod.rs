
cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        pub use self::x86_64::{Ptr, RawPtr};
    } else {
        compile_error! {
            "There is no `smee_alloc` polymorphic pointer implementation for this target"
        }
    }
}



mod traits;
pub use self::traits::Pointer;
