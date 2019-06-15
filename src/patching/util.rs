use super::node::DOMNode;
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Display;
use std::fmt;
use std::hash::{Hash, Hasher};

use wasm_bindgen::JsValue;
use web_sys;

// Efficient append-only string builder for reducing reallocations
pub struct Appender {
	buffers: Vec<String>,
}

impl Appender {
	pub fn new() -> Self {
		return Appender {
			buffers: vec![String::with_capacity(64)],
		};
	}

	fn assert_cap(&mut self, append_size: usize) {
		let buf = self.buffers.last().unwrap();
		let cap = buf.capacity();
		let len = buf.len();
		if len + append_size > cap {
			self.buffers.push(String::with_capacity(cap * 2));
		}
	}

	// Dump all partial buffers into whole string
	pub fn dump(&mut self) -> String {
		self.buffers.concat()
	}

	fn last_mut(&mut self) -> &mut String {
		self.buffers.last_mut().unwrap()
	}
}

impl fmt::Write for Appender {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.assert_cap(s.len());
		self.last_mut().write_str(s)
	}

	fn write_char(&mut self, c: char) -> fmt::Result {
		self.assert_cap(1);
		self.last_mut().write_char(c)
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

// Overrides hashing method.
// The default hashing method for *const is conversion to usize.
#[derive(PartialEq, Eq)]
struct ValuePointer<T: Eq + Hash + Clone + super::WriteHTMLTo>(*const T);

impl<T: Eq + Hash + Clone + super::WriteHTMLTo> Hash for ValuePointer<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		unsafe { (*self.0).hash(state) };
	}
}

// Bidirectional lookup map for <usize,T> with no key (or value) removal.
// Stores values as pointers to avoid copies.
//
// TODO: Test this
#[derive(Default)]
pub struct PointerTokenMap<T: Eq + Hash + Clone + super::WriteHTMLTo> {
	forward: HashMap<u16, ValuePointer<T>>,
	inverted: HashMap<ValuePointer<T>, u16>,
}

impl<T: Eq + Hash + Clone + super::WriteHTMLTo> PointerTokenMap<T> {
	// Get key token for a value, if it is in the map
	pub fn get_token(&self, value: &T) -> Option<&u16> {
		self.inverted.get(unsafe { std::mem::transmute(value) })
	}

	// Get a reference to value from token, if it is in the map
	pub fn get_value(&self, token: u16) -> &T {
		self.forward
			.get(&token)
			.map(|v| unsafe { std::mem::transmute(v) })
			.expect("unset token lookup")
	}

	// Insert new token and value into map
	pub fn insert(&mut self, token: u16, value: T) {
		let ptr = Box::into_raw(Box::new(value)) as *const T;
		self.inverted.insert(ValuePointer(ptr), token);
		self.forward.insert(token, ValuePointer(ptr));
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

// Lazily retrieves an element by its ID
#[derive(Default)]
pub struct LazyElement {
	id: u64,
	element: Option<web_sys::Element>,
}

impl LazyElement {
	pub fn new(id: u64) -> Self {
		Self {
			id: id,
			element: None,
		}
	}

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

impl TryFrom<&DOMNode> for LazyElement {
	type Error = JsValue;

	// Create a fresh element not inserted into the DOM from DOMNode HTML
	fn try_from(node: &DOMNode) -> Result<Self, JsValue> {
		let el = document().create_element("div")?;
		el.set_outer_html(&node.html()?);
		Ok(Self {
			id: node.id,
			element: Some(el),
		})
	}
}

// Cast Rust error to JSValue to be thrown as exception
pub fn cast_error<T: Display>(e: T) -> JsValue {
	JsValue::from(format!("{}", e))
}
