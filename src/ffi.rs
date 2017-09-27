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

// Cast to "borrowed" C string. Rust retains the ownership of the source.
pub fn to_borrowed_string<T: Into<Vec<u8>>>(s: T) -> *const c_char {
	CString::new(s).unwrap().as_ptr()
}
