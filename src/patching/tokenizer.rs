use std::collections::HashMap;
use std::convert::AsRef;
use std::fmt;
use std::hash::Hash;

// Storage for small (len <= 15) strings without allocating extra heap memory
#[derive(PartialEq, Eq, Hash, Clone)]
struct ArrayString {
	length: u8,
	arr: [u8; 15],
}

impl ArrayString {
	pub fn new(s: &str) -> Self {
		let mut arr: [u8; 15] = Default::default();
		arr.copy_from_slice(s.as_bytes());
		Self {
			length: s.len() as u8,
			arr: arr,
		}
	}
}

impl AsRef<str> for ArrayString {
	fn as_ref(&self) -> &str {
		std::str::from_utf8(&self.arr).unwrap()
	}
}

// Type is convertible to and from string reference and can be used as a key in
// a hash map
trait Value: Eq + Hash + Clone + AsRef<str> {}

impl Value for ArrayString {}
impl Value for String {}

// ID->string map with forward and inverted lookup
struct ValueMap<V: Value> {
	mask_high_bit: bool,
	forward: HashMap<u64, V>,
	inverted: HashMap<V, u64>,
}

impl<V: Value> ValueMap<V> {
	pub fn new(mask_high_bit: bool) -> Self {
		ValueMap {
			mask_high_bit: mask_high_bit,
			forward: HashMap::new(),
			inverted: HashMap::new(),
		}
	}

	// Get key token for a string, registering a new token, if not found.
	pub fn get_id(&mut self, v: V) -> u64 {
		if let Some(id) = self.inverted.get(&v) {
			return *id;
		}

		static mut ID_COUNTER: u64 = 0;
		let mut k = unsafe {
			ID_COUNTER += 1;
			ID_COUNTER
		};
		if self.mask_high_bit {
			k |= (1 << 63);
		}
		self.forward.insert(k, v.clone());
		self.inverted.insert(v, k);
		return k;
	}

	// Lookup string by key and write it to w
	pub fn write_str<W: fmt::Write>(&self, k: u64, w: &mut W) -> fmt::Result {
		w.write_str(self.forward.get(&k).unwrap().as_ref())
	}
}

// Contains id->string and string->id mappings
struct Registry {
	small: ValueMap<ArrayString>,
	large: ValueMap<String>,
}

impl Registry {
	fn new() -> Self {
		Self {
			small: ValueMap::new(false),
			large: ValueMap::new(true),
		}
	}

	// Get key for string
	pub fn get_id(&mut self, s: &str) -> u64 {
		if s.len() <= 15 {
			self.small.get_id(ArrayString::new(s))
		} else {
			self.large.get_id(String::from(s))
		}
	}

	// Lookup string by key and write it to w
	pub fn write_str<W: fmt::Write>(&self, k: u64, w: &mut W) -> fmt::Result {
		if k & (1 << 63) == 0 {
			self.small.write_str(k, w)
		} else {
			self.large.write_str(k, w)
		}
	}
}
