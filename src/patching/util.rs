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
#[derive(Default)]
pub struct TokenMap<T: TokenValue> {
	forward: HashMap<u16, T>,
	inverted: HashMap<T, u16>,
}

impl<T: TokenValue> TokenMap<T> {
	// Get key token for a value, if it is in the map
	pub fn get_token(&self, value: &T) -> Option<&u16> {
		self.inverted.get(value)
	}

	// Insert new token and value into map
	pub fn insert(&mut self, token: u16, value: T) {
		self.forward.insert(token, value.clone());
		self.inverted.insert(value, token);
	}

	// Lookup value by token and write to w
	pub fn write_to<W: fmt::Write>(
		&self,
		token: u16,
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
#[derive(Default)]
pub struct PointerTokenMap<T: TokenValue> {
	forward: HashMap<u16, ValuePointer<T>>,
	inverted: HashMap<ValuePointer<T>, u16>,
}

impl<T: TokenValue> PointerTokenMap<T> {
	// Get key token for a value, if it is in the map
	pub fn get_token(&self, value: &T) -> Option<&u16> {
		self.inverted.get(unsafe { std::mem::transmute(value) })
	}

	// Insert new token and value into map
	pub fn insert(&mut self, token: u16, value: T) {
		let ptr = Box::into_raw(Box::new(value)) as *const T;
		self.inverted.insert(ValuePointer(ptr), token);
		self.forward.insert(token, ValuePointer(ptr));
	}

	// Lookup value by token and write to w
	pub fn write_to<W: fmt::Write>(
		&self,
		token: u16,
		w: &mut W,
	) -> fmt::Result {
		match self.forward.get(&token) {
			Some(v) => unsafe { (*v.0).write_to(w) },
			None => panic!("unset token lookup: {}", token),
		}
	}
}

// Generates u16 IDs with optional highest bit flagging
#[derive(Default)]
pub struct IDGenerator {
	counter: u16,
}

impl IDGenerator {
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
