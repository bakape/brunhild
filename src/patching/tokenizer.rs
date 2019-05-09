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

impl util::TokenValue for ArrayString {
	fn write_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		w.write_str(std::str::from_utf8(&self.arr).unwrap())
	}
}

impl util::TokenValue for String {
	fn write_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		w.write_str(&self)
	}
}

// Contains id->string and string->id mappings
struct Registry {
	id_counter: usize,
	small: util::TokenMap<ArrayString>,
	large: util::PointerTokenMap<String>,
}

impl Registry {
	fn new() -> Self {
		Self {
			id_counter: 0,
			small: util::TokenMap::new(),
			large: util::PointerTokenMap::new(),
		}
	}

	fn new_token(&mut self) -> usize {
		self.id_counter += 1;
		self.id_counter
	}

	// Convert string to token
	fn tokenize(&mut self, s: &str) -> usize {
		match s.len() {
			0 => 0, // Don't store empty strings
			1...15 => {
				let v = ArrayString::new(s);
				match self.small.get_token(&v) {
					Some(t) => *t,
					None => {
						let t = self.new_token();
						self.small.insert(t, v);
						t
					}
				}
			}
			_ => {
				let v = String::from(s);
				match self.large.get_token(&v) {
					Some(t) => *t,
					None => {
						let mut t = self.new_token();
						t |= 1 << 63; // Mark highest bit
						self.large.insert(t, v);
						t
					}
				}
			}
		}
	}

	// Lookup string by token and write it to w
	fn write_str<W: fmt::Write>(&self, k: usize, w: &mut W) -> fmt::Result {
		if k == 0 {
			Ok(())
		} else {
			if k & (1 << 63) == 0 {
				self.small.write_to(k, w)
			} else {
				self.large.write_to(k, w)
			}
		}
	}
}

// Convert string to token
pub fn tokenize(s: &str) -> usize {
	util::with_global(&REGISTRY, |r| r.tokenize(s))
}

// Lookup token and write value to w
pub fn write_to<W: fmt::Write>(k: usize, w: &mut W) -> fmt::Result {
	util::with_global(&REGISTRY, |r| r.write_str(k, w))
}
