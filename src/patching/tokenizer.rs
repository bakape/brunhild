// Storage for small (len <= 15) strings without allocating extra heap memory
struct ArrayString {
	length: u8,
	arr: [u8; 15],
}

impl ArrayString {
	pub fn new(s: &str) -> Self {
		let mut arr: [u8; 15] = Default::default();
		arr.copy_from_slice(s.as_bytes());
		ArrayString {
			length: s.len() as u8,
			arr: arr,
		}
	}

	pub fn as_str(&self) -> &str {
		std::str::from_utf8(&self.arr).unwrap()
	}
}
