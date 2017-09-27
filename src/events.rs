use node::Attrs;
use serde_json;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr::null_mut;

static mut HANDLER_ID: u64 = 0;
thread_local!{
	static EVENTS: RefCell<HashMap<Key, Vec<Val>>> =
		RefCell::new(HashMap::new());
}

#[derive(PartialEq, Eq, Hash, Debug)]
struct Key {
	event_type: String,
	selector: String,
}

struct Val {
	id: u64, // used to unbind a handler
	handler: Handler,
}

// Function that handles a DOM event
pub type Handler = fn(&Attrs);

fn with_events<F, R>(func: F) -> R
where
	F: FnOnce(&mut HashMap<Key, Vec<Val>>) -> R,
{
	EVENTS.with(|r| func(r.borrow_mut().borrow_mut()))
}

// Add an event handler.
// Events are matched by event type and optional target selector.
// Returns ID, which can be used to remove the event handler.
pub fn add_listener(typ: &str, selector: &str, handler: Handler) -> u64 {
	let key = Key {
		event_type: String::from(typ),
		selector: String::from(selector),
	};

	let id = unsafe { HANDLER_ID };
	unsafe { HANDLER_ID += 1 };
	let val = Val { id, handler };

	with_events(|e| {
		let has = e.contains_key(&key);
		if has {
			// This will always be true. Need it to work around the borrow
			// checker.
			if let Some(v) = e.get_mut(&key) {
				v.push(val);
			}
		} else {
			e.insert(key, vec![val]);
		}
	});

	to_C_string!(typ, {
		to_C_string!(selector, {
			unsafe { register_listener(typ, selector) };
		});
	});

	id
}

// Remove event listener, if it exists
pub fn remove_listener(id: u64) {
	with_events(|e| {
		e.retain(|key, vals| match vals.iter().find(|v| v.id == id) {
			None => true,
			Some(_) => {
				// Remove from Rust side
				vals.retain(|v| v.id != id);

				let retain = vals.len() != 0;

				// Remove from JS side, only when no handlers of this type left
				if !retain {
					let c_type =
						CString::new(key.event_type.clone()).unwrap().as_ptr();
					let c_selector =
						CString::new(key.selector.clone()).unwrap().as_ptr();
					unsafe { unregister_listener(c_type, c_selector) };
				}

				retain
			}
		})
	});
}

extern "C" {
	fn register_listener(typ: *const c_char, selector: *const c_char);
	fn unregister_listener(typ: *const c_char, selector: *const c_char);
}

// Route a caught event from the JS side
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn delegate_event(
	typ: *mut c_char,
	selector: *mut c_char,
	attrs: *mut c_char,
) {
	let key = Key {
		event_type: from_C_string!(typ),
		selector: if selector != null_mut() {
			from_C_string!(selector)
		} else {
			String::new()
		},
	};
	with_events(|e| match e.get(&key) {
		None => {
			panic!(format!("inconsistent event key: {:?}", key));
		}
		Some(vals) => {
			let attrs: Attrs = serde_json::from_str(&from_C_string!(attrs))
				.unwrap();
			for v in vals {
				(v.handler)(&attrs);
			}
		}
	});
}
