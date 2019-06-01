use super::node::{DOMNode, Node};
use super::util;
use std::cell::RefCell;
use std::rc::Rc;

// Current state of the PENDING tree in comparison to the DOM tree
static mut DIRTY: bool = false;

thread_local! {
	// Tree with changes pending for flush into the DOM
	static PENDING: RefCell<Node> = Default::default();

	// Tree representing the current state of the DOM
	static DOM: RefCell<DOMNode> = Default::default();
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

// Provides methods for manipulating a Node and its subtree
#[derive(Default)]
pub struct Handle {
	// ID of node the handle is referencing
	id: u64,
	// TODO: Lookup cache
}

impl Handle {
	// Queue pending patches for handled node and its subtree
	pub fn patch(new: Node) {
		unsafe { DIRTY = true };
		unimplemented!()
	}

	// TODO: Event handling
}
