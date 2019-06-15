use super::tokenizer;
use super::util;
use super::WriteHTMLTo;
use std::cell::RefCell;
use std::collections::HashSet;
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

impl Into<HashSet<u16>> for ArrayClassSet {
	fn into(self) -> HashSet<u16> {
		let mut set = HashSet::with_capacity(4);
		for id in self.0.iter() {
			if *id == 0 {
				break;
			}
			set.insert(*id);
		}
		return set;
	}
}

impl super::WriteHTMLTo for ArrayClassSet {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		for (i, id) in self.0.iter().enumerate() {
			if *id == 0 {
				break;
			}
			if i != 0 {
				w.write_char(' ')?;
			}
			tokenizer::get_value(*id, |s| w.write_str(s))?;
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

impl Into<HashSet<u16>> for VectorClassSet {
	fn into(self) -> HashSet<u16> {
		self.0.into_iter().collect()
	}
}

impl super::WriteHTMLTo for VectorClassSet {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		for (i, id) in self.0.iter().enumerate() {
			if i != 0 {
				w.write_char(' ')?;
			}
			tokenizer::get_value(*id, |s| w.write_str(s))?;
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
	// Convert class set of strings to token.
	// Including duplicates is the caller's fault.
	fn tokenize(&mut self, set: &[&str]) -> u16 {
		let mut set: Vec<u16> =
			set.into_iter().map(|s| tokenizer::tokenize(s)).collect();
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

	// Lookup class set by token and write it to w
	fn write_html_to<W: fmt::Write>(&self, k: u16, w: &mut W) -> fmt::Result {
		if util::IDGenerator::is_flagged(k) {
			self.large.get_value(k).write_html_to(w)
		} else {
			self.small.get_value(k).write_html_to(w)
		}
	}
}

// Convert class set of strings to token
pub fn tokenize(set: &[&str]) -> u16 {
	util::with_global_mut(&REGISTRY, |r| r.tokenize(set))
}

// // Lookup class set by token and write it to w
pub fn write_html_to<W: fmt::Write>(k: u16, w: &mut W) -> fmt::Result {
	util::with_global_mut(&REGISTRY, |r| r.write_html_to(k, w))
}
