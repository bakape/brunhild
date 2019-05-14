use super::tokenizer;
use super::util;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::convert::From;
use std::fmt;

thread_local! {
	static REGISTRY: RefCell<util::PointerTokenMap<ClassSet>>
		= RefCell::new(util::PointerTokenMap::new());
}

// TODO: token -> class_set for modifying sets
// TODO: class_set -> token for modified sets
// TODO: Write to fmt::Write using token

// A set of tokenized classes
#[derive(PartialEq, Eq, Hash, Clone)]
struct ClassSet {
	set: BTreeSet<usize>,
}

impl util::TokenValue for ClassSet {
	fn write_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		w.write_str("class=\"")?;

		let mut first = true;
		for v in self.set.iter() {
			if first {
				w.write_char(' ')?;
			} else {
				first = false;
			}
			tokenizer::write_to(*v, w)?;
		}

		w.write_char('"')
	}
}

impl ClassSet {
	pub fn new() -> Self {
		Self {
			set: BTreeSet::new(),
		}
	}

	pub fn add(&mut self, class: &str) {
		self.set.insert(tokenizer::tokenize(class));
	}

	pub fn remove(&mut self, class: &str) {
		self.set.remove(&tokenizer::tokenize(class));
	}
}

impl From<&str> for ClassSet {
	fn from(s: &str) -> Self {
		Self {
			set: s
				.split(' ')
				.filter(|x| x.len() != 0)
				.map(|x| tokenizer::tokenize(x))
				.collect(),
		}
	}
}
