use super::util;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::AsRef;
use std::fmt;
use std::hash::Hash;

thread_local! {
	static REGISTRY: RefCell<Registry> = Default::default();
}

// Storage for small (len <= 15) strings without allocating extra heap memory
#[derive(Default, PartialEq, Eq, Hash, Clone)]
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
#[derive(Default)]
struct Registry {
	id_gen: util::IDGenerator,
	small: util::TokenMap<ArrayString>,
	large: util::PointerTokenMap<String>,
}

impl Registry {
	// Convert string to token
	fn tokenize(&mut self, s: &str) -> u16 {
		match s.len() {
			0 => 0, // Don't store empty strings
			1...15 => {
				let v = ArrayString::new(s);
				match self.small.get_token(&v) {
					Some(t) => *t,
					None => {
						let t = self.id_gen.new_id(false);
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
						let t = self.id_gen.new_id(true);
						self.large.insert(t, v);
						t
					}
				}
			}
		}
	}

	// Lookup string by token and write it to w
	fn write_to<W: fmt::Write>(&self, k: u16, w: &mut W) -> fmt::Result {
		if k == 0 {
			Ok(())
		} else {
			if util::IDGenerator::is_flagged(k) {
				self.large.write_to(k, w)
			} else {
				self.small.write_to(k, w)
			}
		}
	}
}

// Convert string to token
pub fn tokenize(s: &str) -> u16 {
	util::with_global(&REGISTRY, |r| r.tokenize(s))
}

// Lookup token and write value to w
pub fn write_to<W: fmt::Write>(k: u16, w: &mut W) -> fmt::Result {
	util::with_global(&REGISTRY, |r| r.write_to(k, w))
}
