


/// A macro to create a `CString` from a string literal.
#[macro_export]
macro_rules! cstring {
    ($s:expr) => {
        ::std::ffi::CString::new($s).unwrap()
    };
}