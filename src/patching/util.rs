use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

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
}

impl fmt::Write for Appender {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.assert_cap(s.len());
		self.buffers.last_mut().unwrap().write_str(s)
	}

	fn write_char(&mut self, c: char) -> fmt::Result {
		self.assert_cap(1);
		self.buffers.last_mut().unwrap().write_char(c)
	}
}

// Run function with global variable mutable access
pub fn with_global<F, R, G>(
	global: &'static std::thread::LocalKey<std::cell::RefCell<G>>,
	func: F,
) -> R
where
	F: FnOnce(&mut G) -> R,
{
	global.with(|r| func(r.borrow_mut().borrow_mut()))
}

// Value stored in TokenMap
pub trait TokenValue: Eq + Hash + Clone {
	// Write representation to w
	fn write_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result;
}

// Bidirectional lookup map for <usize,T> with no key (or value) removal
pub struct TokenMap<T: TokenValue> {
	forward: HashMap<usize, T>,
	inverted: HashMap<T, usize>,
}

impl<T: TokenValue> TokenMap<T> {
	pub fn new() -> Self {
		Self {
			forward: HashMap::new(),
			inverted: HashMap::new(),
		}
	}

	// Get key token for a value, if it is in the map
	pub fn get_token(&self, value: &T) -> Option<&usize> {
		self.inverted.get(value)
	}

	// Insert new token and value into map
	pub fn insert(&mut self, token: usize, value: T) {
		self.forward.insert(token, value.clone());
		self.inverted.insert(value, token);
	}

	// Lookup value by token and write to w
	pub fn write_to<W: fmt::Write>(
		&self,
		token: usize,
		w: &mut W,
	) -> fmt::Result {
		match self.forward.get(&token) {
			Some(v) => v.write_to(w),
			None => panic!("unset token lookup: {}", token),
		}
	}
}

// Overrides hashing method.
// The default hashing method for *const is conversion to usize.
#[derive(PartialEq, Eq)]
struct ValuePointer<T: TokenValue>(*const T);

impl<T: TokenValue> Hash for ValuePointer<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		unsafe { (*self.0).hash(state) };
	}
}

// Bidirectional lookup map for <usize,T> with no key (or value) removal.
// Stores values as pointers to avoid copies.
pub struct PointerTokenMap<T: TokenValue> {
	forward: HashMap<usize, ValuePointer<T>>,
	inverted: HashMap<ValuePointer<T>, usize>,
}

impl<T: TokenValue> PointerTokenMap<T> {
	pub fn new() -> Self {
		Self {
			forward: HashMap::new(),
			inverted: HashMap::new(),
		}
	}

	// Get key token for a value, if it is in the map
	pub fn get_token(&self, value: &T) -> Option<&usize> {
		self.inverted.get(unsafe { std::mem::transmute(value) })
	}

	// Insert new token and value into map
	pub fn insert(&mut self, token: usize, value: T) {
		let ptr = Box::into_raw(Box::new(value)) as *const T;
		self.inverted.insert(ValuePointer(ptr), token);
		self.forward.insert(token, ValuePointer(ptr));
	}

	// Lookup value by token and write to w
	pub fn write_to<W: fmt::Write>(
		&self,
		token: usize,
		w: &mut W,
	) -> fmt::Result {
		match self.forward.get(&token) {
			Some(v) => unsafe { (*v.0).write_to(w) },
			None => panic!("unset token lookup: {}", token),
		}
	}
}
