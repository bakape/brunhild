use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;

use std::fmt;
use std::fmt::Display;
use std::hash::Hash;
use wasm_bindgen::JsValue;
use web_sys;

// Efficient append-only string builder for reducing reallocations
pub struct Appender {
	i: usize,
	buffers: Vec<String>,
}

impl Appender {
	pub fn new() -> Self {
		return Appender {
			i: 0,
			buffers: vec![String::with_capacity(64)],
		};
	}

	fn current(&mut self) -> &mut String {
		&mut self.buffers[self.i]
	}

	fn assert_cap(&mut self, append_size: usize) {
		let buf = self.current();
		let cap = buf.capacity();
		if buf.len() + append_size > cap {
			if self.i == self.buffers.len() - 1 {
				self.buffers.push(String::with_capacity(cap * 2));
			} else {
				self.i += 1;
			}
		}
	}

	// Clear all contents, but keep allocated memory for reuse
	pub fn clear(&mut self) {
		self.i = 0;
		for b in self.buffers.iter_mut() {
			b.clear()
		}
	}

	// Dump all partial buffers into whole string
	pub fn dump(&mut self) -> String {
		self.buffers.concat()
	}
}

impl fmt::Write for Appender {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.assert_cap(s.len());
		self.current().write_str(s)
	}

	fn write_char(&mut self, c: char) -> fmt::Result {
		self.assert_cap(1);
		self.current().write_char(c)
	}
}

// Lazily retrieves an element by its ID
#[derive(Default)]
pub struct LazyElement {
	pub id: u64,
	element: Option<web_sys::Element>,
}

impl LazyElement {
	// Retrieve JS element reference or cached value
	pub fn get(&mut self) -> Result<web_sys::Element, JsValue> {
		match &mut self.element {
			Some(el) => Ok(el.clone()),
			None => {
				match document().get_element_by_id(&format!("bh-{}", self.id)) {
					Some(el) => {
						self.element = Some(el.clone());
						Ok(el)
					}
					None => {
						Err(format!("element not found: bh-{}", self.id).into())
					}
				}
			}
		}
	}
}

// Run function with global variable immutable access
pub fn with_global<F, R, G>(
	global: &'static std::thread::LocalKey<std::cell::RefCell<G>>,
	func: F,
) -> R
where
	F: FnOnce(&G) -> R,
{
	global.with(|r| func(r.borrow().borrow()))
}

// Run function with global variable mutable access
pub fn with_global_mut<F, R, G>(
	global: &'static std::thread::LocalKey<std::cell::RefCell<G>>,
	func: F,
) -> R
where
	F: FnOnce(&mut G) -> R,
{
	global.with(|r| func(r.borrow_mut().borrow_mut()))
}

// Bidirectional lookup map for <usize,T> with no key (or value) removal
#[derive(Default)]
pub struct TokenMap<T: Eq + Hash + Clone> {
	forward: HashMap<u16, T>,
	inverted: HashMap<T, u16>,
}

impl<T: Eq + Hash + Clone> TokenMap<T> {
	// Get key token for a value, if it is in the map
	pub fn get_token(&self, value: &T) -> Option<&u16> {
		self.inverted.get(value)
	}

	// Get a reference to value from token, if it is in the map
	pub fn get_value(&self, token: u16) -> &T {
		self.forward.get(&token).expect("unset token lookup")
	}

	// Insert new token and value into map
	pub fn insert(&mut self, token: u16, value: T) {
		self.forward.insert(token, value.clone());
		self.inverted.insert(value, token);
	}
}

// Generates u16 IDs with optional highest bit flagging
#[derive(Default)]
pub struct IDGenerator {
	counter: u16,
}

impl IDGenerator {
	pub fn new(start_from: u16) -> Self {
		Self {
			counter: start_from,
		}
	}

	// Create new ID  with optional highest bit flagging
	pub fn new_id(&mut self, flag_highest: bool) -> u16 {
		self.counter += 1;
		let mut id = self.counter;
		if flag_highest {
			id |= 1 << 15;
		}
		return id;
	}

	// Shorthand for checking highest bit being flagged
	#[inline]
	pub fn is_flagged(id: u16) -> bool {
		return id & (1 << 15) != 0;
	}
}

// HTML-Escape a string
pub fn html_escape(s: &str) -> String {
	let mut escaped = String::with_capacity(s.len());
	for ch in s.chars() {
		match ch {
			'&' => escaped += "&amp;",
			'\'' => {
				escaped += "&#39;"; // "&#39;" is shorter than "&apos;"
			}
			'<' => {
				escaped += "&lt;";
			}
			'>' => {
				escaped += "&gt;";
			}
			'"' => {
				escaped += "&#34;"; // "&#34;" is shorter than "&quot;"
			}
			_ => {
				escaped.push(ch);
			}
		};
	}
	escaped
}

// Get JS window global
pub fn window() -> web_sys::Window {
	web_sys::window().expect("no window global")
}

// Get page document
pub fn document() -> web_sys::Document {
	window().document().expect("no document on window")
}

// Cast Rust error to JSValue to be thrown as exception
pub fn cast_error<T: Display>(e: T) -> JsValue {
	JsValue::from(format!("{}", e))
}

// Able to write itself as HTML to w
pub trait WriteHTMLTo {
	fn write_html_to<W: fmt::Write>(&mut self, w: &mut W) -> fmt::Result;
}
