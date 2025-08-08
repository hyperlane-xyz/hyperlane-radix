#[macro_export]
macro_rules! format_error {
    ($($arg:tt)*) => {{
        format!("{}: {}", <Self as ComponentState>::BLUEPRINT_NAME, format!($($arg)*))
    }}
}

#[macro_export]
macro_rules! panic_error {
    ($($arg:tt)*) => {{
        panic!("{}: {}", <Self as ComponentState>::BLUEPRINT_NAME, format!($($arg)*))
    }}
}
