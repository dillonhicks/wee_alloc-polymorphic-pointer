use Ptr;

/// Anything that can be initialized with a `const` value.
pub(crate) trait ConstInit {
    /// The `const` default initializer value for `Self`.
    const INIT: Self;
}

impl<T> ConstInit for *const T {
    const INIT: Self = 0 as *mut _;
}

impl<T> ConstInit for Ptr<T> {
    const INIT: Self = Ptr::null();
}
