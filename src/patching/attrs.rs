use super::tokenizer;
use super::util;
use std::collections::HashMap;
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
#[derive(Default)]
pub struct Attrs {
	map: HashMap<u16, Value>,
}

// Contains a value stored in one of three storage methods for attribute values
enum Value {
	// Tokenized string value
	StringToken(u16),

	// Untokenized string. Used to store values too dynamic to benefit from
	// tokenization in most use cases.
	Untokenized(String),
}

impl Attrs {
	// Create new attribute map from any key-value pair iterator-convertible
	pub fn new<'a, T>(attrs: T) -> Self
	where
		T: IntoIterator<Item = (&'a str, &'a str)>,
	{
		let mut s = Self::default();
		for (k, v) in attrs.into_iter() {
			s.set(k, v);
		}
		return s;
	}

	// Sets an attribute value of a Node.
	// Setting element "id" or "class" attributes is not supported here and does
	// nothing.
	pub fn set(&mut self, key: &str, val: &str) {
		match key {
			"id" => (),
			"class" => (),
			_ => {
				self.map.insert(
					tokenizer::tokenize(key),
					if val == "" {
						Value::StringToken(0)
					} else {
						match TOKENIZABLE_VALUES.binary_search(&key) {
							Ok(_) => {
								Value::StringToken(tokenizer::tokenize(val))
							}
							_ => Value::Untokenized(String::from(val)),
						}
					},
				);
			}
		};
	}

	// Remove attribute from node
	pub fn remove(&mut self, key: &str) {
		self.map.remove(&tokenizer::tokenize(key));
	}

	// Clear all attributes
	pub fn clear(&mut self) {
		self.map.clear();
	}
}

impl fmt::Write for Attrs {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		unimplemented!()
	}
}
