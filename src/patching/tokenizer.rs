use super::util;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::AsRef;
use std::fmt;
use std::hash::Hash;

thread_local! {
	static REGISTRY: RefCell<Registry> = RefCell::new(Registry::new());
}

// Storage for small (len <= 15) strings without allocating extra heap memory
#[derive(PartialEq, Eq, Hash, Clone)]
struct ArrayString {
	length: u8,
	arr: [u8; 15],
}

impl ArrayString {
	fn new(s: &str) -> Self {
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
	fn new(mask_high_bit: bool) -> Self {
		ValueMap {
			mask_high_bit: mask_high_bit,
			forward: HashMap::new(),
			inverted: HashMap::new(),
		}
	}

	// Get key token for a string, registering a new token, if not found.
	fn tokenize(&mut self, v: V) -> u64 {
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

	// Lookup string by token and write it to w
	fn write_str<W: fmt::Write>(&self, k: u64, w: &mut W) -> fmt::Result {
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

	// Convert string to token
	fn tokenize(&mut self, s: &str) -> u64 {
		match s.len() {
			0 => 0, // Don't store empty strings
			1...15 => self.small.tokenize(ArrayString::new(s)),
			_ => self.large.tokenize(String::from(s)),
		}
	}

	// Lookup string by token and write it to w
	fn write_str<W: fmt::Write>(&self, k: u64, w: &mut W) -> fmt::Result {
		if k == 0 {
			Ok(())
		} else {
			if k & (1 << 63) == 0 {
				self.small.write_str(k, w)
			} else {
				self.large.write_str(k, w)
			}
		}
	}
}

// Convert string to token
pub fn tokenize(s: &str) -> u64 {
	util::with_global(&REGISTRY, |r| r.tokenize(s))
}

// Lookup string by token and write it to w
pub fn write_str<W: fmt::Write>(k: u64, w: &mut W) -> fmt::Result {
	util::with_global(&REGISTRY, |r| r.write_str(k, w))
}
