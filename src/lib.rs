extern crate libc;
extern crate serde_json;

pub mod ffi;
mod mutations;
mod node;
pub mod events;

pub use self::events::*;
pub use self::mutations::*;
pub use self::node::{Attrs, Node, new_id};

// Register flush_mutations() with emscripten event loop
pub fn start() {
	unsafe {
		emscripten_set_main_loop(mutations::flush_mutations, 0, 0);
	}
}

extern "C" {
	pub fn emscripten_set_main_loop(
		func: extern "C" fn(),
		fps: libc::c_int,
		infinite: libc::c_int,
	);
}
