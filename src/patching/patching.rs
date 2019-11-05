use super::node::{DOMNode, Handle, Node};
use super::util;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;
use wasm_bindgen::{prelude, JsCast, JsValue};

thread_local! {
	// Tree with changes pending for flush into the DOM
	pub static PENDING: RefCell<Node> = Default::default();

	// Tree representing the current state of the DOM
	pub static DOM: RefCell<DOMNode> = Default::default();

	// JS function closure for patching function
	static PATCH_FUNCTION: RefCell<
		Option<prelude::Closure<dyn Fn() -> Result<(), JsValue>>>,
	> = RefCell::new(None);
}

// A patch is scheduled for the next animation frame
static mut SCHEDULED: bool = false;

// Set the root Node to be attached directly under <body>.
// Overwrites the current state of the entire DOM tree.
pub fn set_root(root: Node) -> Rc<Handle> {
	schedule_patch();
	util::with_global_mut(&PENDING, |r| {
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
		util::with_global_mut(&PATCH_FUNCTION, |f| {
			*f = Some(prelude::Closure::wrap(
				Box::new(patch) as Box<dyn Fn() -> Result<(), JsValue>>
			))
		});
	});

	if !unsafe { SCHEDULED } {
		unsafe { SCHEDULED = true };
		util::with_global_mut(&PATCH_FUNCTION, |f| {
			util::window()
				.request_animation_frame(
					f.as_ref().unwrap().as_ref().unchecked_ref(),
				)
				.unwrap();
		})
	}
}

// Diff and patch pending changes to DOM
fn patch() -> Result<(), JsValue> {
	unsafe { SCHEDULED = false };
	util::with_global_mut(&DOM, |dom_root| {
		util::with_global_mut(&PENDING, |pending_root| {
			if dom_root.id == 0 {
				// Initial root node
				*dom_root = pending_root.into();
				util::document()
					.body()
					.expect("no document body")
					.set_inner_html(&dom_root.html()?);
				Ok(())
			} else {
				dom_root.diff(pending_root)
			}
		})
	})
}
