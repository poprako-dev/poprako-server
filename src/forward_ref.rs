/// Abstracts marker-specific access to an inner type behind a wrapper.
///
/// The marker parameter lets one wrapper forward different trait families to
/// different fields without exposing those fields directly.
pub trait ForwardRef<M> {
    /// The inner type this value forwards to for marker `M`.
    type Target: ?Sized;

    /// Returns a shared reference to the target selected by marker `M`.
    fn forward_ref(&self) -> &Self::Target;
}

/// Implements [`ForwardRef`] for one or more marker types using the same field.
#[macro_export]
macro_rules! impl_forward_ref {
    ($source:ty => $target:ty, $field:ident, $($marker:ty),+ $(,)?) => {
        $(
            impl $crate::ForwardRef<$marker> for $source {
                type Target = $target;

                fn forward_ref(&self) -> &$target {
                    &self.$field
                }
            }
        )+
    };
}
