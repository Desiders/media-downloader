#[macro_export]
macro_rules! impl_from_format {
    ($from_type:ty => $to_type:ty { $($field:ident),+ $(,)? }) => {
        impl From<$from_type> for $to_type {
            fn from(value: $from_type) -> Self {
                Self {
                    $($field: value.$field,)+
                }
            }
        }
    };
}
