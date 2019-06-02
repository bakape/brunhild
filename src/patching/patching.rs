use super::node::{DOMNode, Handle, Node};
use super::util;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;

use wasm_bindgen::{prelude, JsCast};
use web_sys;

thread_local! {
	// Tree with changes pending for flush into the DOM
	pub static PENDING: RefCell<Node> = Default::default();

	// Tree representing the current state of the DOM
	pub static DOM: RefCell<DOMNode> = Default::default();

	// JS function closure for patching function
	static PATCH_FUNCTION: RefCell<Option<prelude::Closure<Fn()>>>
		= RefCell::new(None);
}

// Set the root Node to be attached directly under <body>.
// Overwrites the current state of the entire DOM tree.
pub fn set_root(root: Node) -> Rc<Handle> {
	schedule_patch();
	util::with_global(&PENDING, |r| {
		*r = root;
		r.take_handle()
	})
}

// Schedule a diff and patch of the DOM on the next animation frame, if not
// scheduled already.
pub fn schedule_patch() {
	// Create a JS function for the patch function
	static CREATE_CLOSURE: Once = Once::new();
	CREATE_CLOSURE.call_once(|| {
		util::with_global(&PATCH_FUNCTION, |f| {
			*f = Some(prelude::Closure::wrap(Box::new(patch) as Box<Fn()>))
		});
	});

	static mut SCHEDULED: bool = false;
	if !unsafe { SCHEDULED } {
		unsafe { SCHEDULED = true };
		util::with_global(&PATCH_FUNCTION, |f| {
			web_sys::window()
				.unwrap()
				.request_animation_frame(
					f.as_ref().unwrap().as_ref().unchecked_ref(),
				)
				.unwrap();
		})
	}
}

// Diff and patch pending changes to DOM
fn patch() {
	unimplemented!()
}
