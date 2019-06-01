use super::tokenizer;
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
#[derive(Default, Clone)]
pub struct Attrs(HashMap<u16, Value>);

// Contains a value stored in one of 2 storage methods for attribute values
#[derive(Clone)]
enum Value {
	// Tokenized string value
	StringToken(u16),

	// Untokenized string. Used to store values too dynamic to benefit from
	// tokenization in most use cases.
	Untokenized(String),
}

impl Attrs {
	// Create empty attribute map
	pub fn new(arr: &[&(&str, &str)]) -> Self {
		let mut s = Self::with_capacity(arr.len());
		for (k, v) in arr.iter() {
			s.set(k, v);
		}
		return s;
	}

	// Create empty attribute map with set capacity
	pub fn with_capacity(capacity: usize) -> Self {
		Self(HashMap::with_capacity(capacity))
	}

	// Sets an attribute value of a Node.
	// Setting element "id" or "class" attributes is not supported here.
	pub fn set(&mut self, key: &str, val: &str) {
		self.0.insert(
			tokenizer::tokenize(key),
			if val == "" {
				Value::StringToken(0)
			} else {
				match TOKENIZABLE_VALUES.binary_search(&key) {
					Ok(_) => Value::StringToken(tokenizer::tokenize(val)),
					_ => Value::Untokenized(String::from(val)),
				}
			},
		);
	}

	// Remove attribute from node
	pub fn remove(&mut self, key: &str) {
		self.0.remove(&tokenizer::tokenize(key));
	}
}

impl super::WriteHTMLTo for Attrs {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		for (k, v) in self.0.iter() {
			w.write_char(' ')?;
			tokenizer::write_html_to(*k, w)?;
			match v {
				Value::StringToken(v) => {
					if *v != 0 {
						w.write_char('"')?;
						tokenizer::write_html_to(*v, w)?;
						w.write_char('"')?;
					}
				}
				Value::Untokenized(s) => {
					w.write_char('"')?;
					w.write_str(&s)?;
					w.write_char('"')?;
				}
			};
		}
		Ok(())
	}
}
