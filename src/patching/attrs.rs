use super::tokenizer;
use super::util;

use std::collections::HashMap;
use std::fmt;
use wasm_bindgen::JsValue;

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
#[derive(Clone, PartialEq, Eq)]
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
	//
	// # Panics
	//
	// Setting element "id" or "class" attributes is not supported. Panics,
	// if key in ("id", "class")
	pub fn set(&mut self, key: &str, val: &str) {
		match key {
			"id" | "class" => {
				panic!(format!(
					"manually setting attribute not supported: {}",
					key
				));
			}
			_ => {
				self.0.insert(
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
		}
	}

	// Diff and patch attributes against new set and write changes to the DOM
	pub fn patch(
		&mut self,
		el: &mut util::LazyElement,
		new: &mut Attrs,
	) -> Result<(), JsValue> {
		// Attributes added or changed
		for (k, v) in new.0.iter() {
			let set = match self.0.get_mut(k) {
				Some(old_v) => {
					if v != old_v {
						*old_v = v.clone();
						true
					} else {
						false
					}
				}
				None => {
					self.0.insert(*k, v.clone());
					true
				}
			};
			if set {
				match el.get() {
					Ok(el) => tokenizer::get_value(*k, |key| match v {
						Value::StringToken(v) => {
							tokenizer::get_value(*v, |value| {
								el.set_attribute(key, value)
							})
						}
						Value::Untokenized(value) => {
							el.set_attribute(key, value)
						}
					}),
					Err(e) => Err(e),
				}?;
			}
		}

		// Attributes removed
		let mut to_remove = Vec::<u16>::new();
		for k in self.0.keys() {
			if !new.0.contains_key(k) {
				continue;
			}
			to_remove.push(*k);
			match el.get() {
				Ok(el) => {
					tokenizer::get_value(*k, |key| el.remove_attribute(key))
				}
				Err(e) => Err(e),
			}?;
		}
		for k in to_remove {
			self.0.remove(&k);
		}

		Ok(())
	}
}

impl super::WriteHTMLTo for Attrs {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		for (k, v) in self.0.iter() {
			tokenizer::get_value(*k, |s| write!(w, " {}", s))?;
			match v {
				Value::StringToken(v) => {
					if *v != 0 {
						tokenizer::get_value(*k, |s| write!(w, "=\"{}\"", s))?;
					}
				}
				Value::Untokenized(s) => {
					write!(w, "=\"{}\"", s)?;
				}
			};
		}
		Ok(())
	}
}
