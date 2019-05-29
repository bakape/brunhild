use super::tokenizer;
use super::util;
use std::cell::RefCell;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::collections::BTreeSet;
use std::fmt;
use std::iter::Iterator;

thread_local! {
	static REGISTRY: RefCell<Registry> = Default::default();
}

// Storage for sorted sets of up to 4 classes without indirection or heap
// allocation
#[derive(Default, PartialEq, Eq, Hash, Clone)]
struct ArrayClassSet([u16; 4]);

impl From<&Vec<u16>> for ArrayClassSet {
	fn from(vec: &Vec<u16>) -> Self {
		let mut cs = ArrayClassSet::default();
		for (i, id) in vec.iter().enumerate() {
			cs.0[i] = *id;
		}
		return cs;
	}
}

impl Into<Vec<u16>> for ArrayClassSet {
	fn into(self) -> Vec<u16> {
		let mut vec = Vec::with_capacity(4);
		for id in self.0.iter() {
			if *id == 0 {
				break;
			}
			vec.push(*id);
		}
		return vec;
	}
}

impl util::TokenValue for ArrayClassSet {
	fn write_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		for (i, id) in self.0.iter().enumerate() {
			if *id == 0 {
				break;
			}
			if i != 0 {
				w.write_char(' ')?;
			}
			tokenizer::write_to(*id, w)?;
		}
		Ok(())
	}
}

// Storage for sorted sets of up to 4+ classes
#[derive(Default, PartialEq, Eq, Hash, Clone)]
struct VectorClassSet(Vec<u16>);

impl VectorClassSet {
	fn new(vec: Vec<u16>) -> Self {
		Self(vec)
	}
}

impl util::TokenValue for VectorClassSet {
	fn write_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		for (i, id) in self.0.iter().enumerate() {
			if i != 0 {
				w.write_char(' ')?;
			}
			tokenizer::write_to(*id, w)?;
		}
		Ok(())
	}
}

// Contains id->class_set and class_set->id mappings
#[derive(Default)]
struct Registry {
	id_gen: util::IDGenerator,
	small: util::TokenMap<ArrayClassSet>,
	large: util::PointerTokenMap<VectorClassSet>,
}

impl Registry {
	// Convert class set to token
	fn tokenize_set(&mut self, mut set: Vec<u16>) -> u16 {
		set.sort();

		match set.len() {
			0 => 0, // Don't store empty class sets
			1...4 => {
				let v = ArrayClassSet::from(&set);
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
				let v = VectorClassSet::new(set);
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

	// Convert class set of strings to token
	fn tokenize<'a, I: Iterator<Item = &'a str>>(&mut self, set: I) -> u16 {
		// Including duplicates is the caller's fault
		self.tokenize_set(set.map(|s| tokenizer::tokenize(s)).collect())
	}

	// // Lookup class set by token and write it to w
	fn write_class_set<W: fmt::Write>(&self, k: u16, w: &mut W) -> fmt::Result {
		w.write_str("class=\"")?;
		if k != 0 {
			if util::IDGenerator::is_flagged(k) {
				self.large.write_to(k, w)?;
			} else {
				self.small.write_to(k, w)?;
			}
		}
		w.write_char('"')
	}

	// Add class to given tokenized set and return new set ID
	fn add_class(src: u16) -> u16 {
		unimplemented!()
	}

	// Remove class from given tokenized set and return new set ID
	fn remove_class(src: u16) -> u16 {
		unimplemented!()
	}
}

// TODO: Macro to generate public functions. Also reuse it in tokenizer.rs.
