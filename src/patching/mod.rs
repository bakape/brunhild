#![allow(unused)]

use std::fmt;

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
