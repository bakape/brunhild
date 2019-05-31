use super::tokenizer;
use super::util;
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
			tokenizer::write_html_to(*id, w)?;
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
			tokenizer::write_html_to(*id, w)?;
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
	fn tokenize<'a, I: IntoIterator<Item = &'a str>>(&mut self, set: I) -> u16 {
		// Including duplicates is the caller's fault
		self.tokenize_set(
			set.into_iter().map(|s| tokenizer::tokenize(s)).collect(),
		)
	}

	// // Lookup class set by token and write it to w
	fn write_html_to<W: fmt::Write>(&self, k: u16, w: &mut W) -> fmt::Result {
		if util::IDGenerator::is_flagged(k) {
			self.large.write_html_to(k, w)
		} else {
			self.small.write_html_to(k, w)
		}
	}

	// Augment existing class set and return new class set ID
	fn augment_class_set<F>(&mut self, token: &mut u16, func: F)
	where
		F: FnOnce(&mut HashSet<u16>),
	{
		macro_rules! get {
			($x:ident) => {
				match self.$x.get_value(*token) {
					Some(v) => Some(v.into()),
					None => None,
					}
			};
		}

		let mut old = match {
			if util::IDGenerator::is_flagged(*token) {
				get!(large)
			} else {
				get!(small)
			}
		} {
			Some(v) => v,
			None => panic!("unregistered class token lookup: {}", token),
		};
		func(&mut old);
		*token = self.tokenize_set(old.into_iter().collect());
	}

	// Add class to given tokenized set and write new set ID to reference
	fn add_class(&mut self, token: &mut u16, class: &str) {
		self.augment_class_set(token, |old| {
			old.insert(tokenizer::tokenize(class));
		});
	}

	// Remove class from given tokenized set and write new set ID to reference
	fn remove_class(&mut self, token: &mut u16, class: &str) {
		self.augment_class_set(token, |old| {
			old.remove(&tokenizer::tokenize(class));
		});
	}
}

// Convert class set of strings to token
pub fn tokenize<'a, I: IntoIterator<Item = &'a str>>(set: I) -> u16 {
	util::with_global(&REGISTRY, |r| r.tokenize(set))
}

// // Lookup class set by token and write it to w
pub fn write_html_to<W: fmt::Write>(k: u16, w: &mut W) -> fmt::Result {
	util::with_global(&REGISTRY, |r| r.write_html_to(k, w))
}

// Add class to given tokenized set and write new set ID to reference
pub fn add_class(token: &mut u16, class: &str) {
	util::with_global(&REGISTRY, |r| r.add_class(token, class));
}

// Remove class from given tokenized set and write new set ID to reference
pub fn remove_class(token: &mut u16, class: &str) {
	util::with_global(&REGISTRY, |r| r.remove_class(token, class));
}
