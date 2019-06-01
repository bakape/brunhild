use super::node::{DOMNode, Handle, Node};
use super::util;
use std::cell::RefCell;
use std::rc::Rc;

// Current state of the PENDING tree in comparison to the DOM tree
pub static mut DIRTY: bool = false;

thread_local! {
	// Tree with changes pending for flush into the DOM
	pub static PENDING: RefCell<Node> = Default::default();

	// Tree representing the current state of the DOM
	pub static DOM: RefCell<DOMNode> = Default::default();
}

// Set the root Node to be attached directly under <body>.
// Overwrites the current state of the entire DOM tree.
pub fn set_root(root: Node) -> Rc<Handle> {
	unsafe { DIRTY = true };
	util::with_global(&PENDING, |r| {
		*r = root;
		r.take_handle()
	})
}
