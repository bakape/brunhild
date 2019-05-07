use super::tokenizer::tokenize;
use super::util;
use std::collections::BTreeMap;
use std::fmt;

// Attribute keys that have limited set of values and thus can have their
// values tokenized.
// Sorted for binary search.
//
// Sourced from:
// https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes
static TOKENIZABLE_VALUES: [&'static str; 34] = [
	"async",
	"autocapitalize",
	"autocomplete",
	"autofocus",
	"autoplay",
	"checked",
	"contenteditable",
	"controls",
	"crossorigin",
	"decoding",
	"defer",
	"dir",
	"disabled",
	"draggable",
	"dropzone",
	"hidden",
	"language",
	"loop",
	"method",
	"multiple",
	"muted",
	"novalidate",
	"open",
	"preload",
	"readonly",
	"referrerpolicy",
	"required",
	"reversed",
	"sandbox",
	"selected",
	"spellcheck",
	"translate",
	"type",
	"wrap",
];

// Compressed attribute storage with manipulation functions
pub struct Attrs {
	map: BTreeMap<u64, Value>,
}

// Contains a value stored in one of three storage methods for attribute values
enum Value {
	// Tokenized string value
	StringToken(u64),

	// Tokenized set of classes
	ClassSet(u64),

	// Untokenized string. Used to store values too dynamic to benefit from
	// tokenization in most use cases.
	Untokenized(String),
}

impl Attrs {
	pub fn new() -> Self {
		Self {
			map: BTreeMap::new(),
		}
	}

	// Sets an attribute value of a Node.
	// Setting element id attributes is not supported and does nothing.
	pub fn set(&mut self, key: &str, val: &str) {
		match key {
			"id" => (),
			// TODO: Forward to class setting method
			"class" => unimplemented!(),
			_ => {
				self.map.insert(
					tokenize(key),
					if val == "" {
						Value::StringToken(0)
					} else {
						match TOKENIZABLE_VALUES.binary_search(&key) {
							Ok(_) => Value::StringToken(tokenize(val)),
							_ => Value::Untokenized(String::from(val)),
						}
					},
				);
			}
		};
	}

	// Remove attribute from node
	pub fn remove(&mut self, key: &str) {
		self.map.remove(&tokenize(key));
	}

	// Clear all attributes
	pub fn clear(&mut self) {
		self.map.clear();
	}

	// Add class to Node class set
	pub fn add_class(&mut self, class: &str) {
		unimplemented!()
	}

	// Remove class from Node class set
	pub fn remove_class(&mut self, class: &str) {
		unimplemented!()
	}
}

impl fmt::Write for Attrs {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		unimplemented!()
	}
}
