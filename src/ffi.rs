use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// Cast "borrowed" C string to &str.
// Lifetime is not actually static and id defined by the C side.
pub fn from_borrowed_string(s: *const c_char) -> &'static str {
	unsafe { CStr::from_ptr(s) }.to_str().unwrap()
}

// Cast owned C string to String
pub fn from_owned_string(s: *mut c_char) -> String {
	unsafe { CString::from_raw(s) }.into_string().unwrap()
}

// Cast to C string and execute with, while keeping the same variable name.
// Needed to make sure the string is not dropped before the C function returns.
#[macro_export]
macro_rules! as_c_string {
	( $var:ident, $fn:expr ) => (
		{
			let $var = ::std::ffi::CString::new($var).unwrap();
			{
				let $var = $var.as_ptr();
				$fn
			}
		}
	)
}
