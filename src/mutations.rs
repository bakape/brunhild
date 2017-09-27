pub use self::externs::get_inner_html;
use std::borrow::BorrowMut;
use std::cell::RefCell;

thread_local! {
	static MUTATIONS: RefCell<Vec<Mutation>> = RefCell::new(Vec::new());
}

// Single buffered mutation to be written to the dom
struct Mutation {
	id: String,
	data: MutationData,
}

// Data of a pending mutation
#[allow(non_camel_case_types)]
enum MutationData {
	// Insertions. Contain HTML strings.
	append(String),
	prepend(String),
	before(String),
	after(String),
	set_inner_html(String),
	set_outer_html(String),

	set_attr(String, Option<String>),
	remove_attr(String),

	// Remove node
	remove,
}

pub fn set_attr(id: &str, key: &str, val: Option<&str>) {
	push_mutation(
		id,
		MutationData::set_attr(
			String::from(key),
			match val {
				Some(v) => Some(String::from(v)),
				None => None,
			},
		),
	);
}

pub fn set_inner_html(parent_id: &str, html: &str) {
	with_mutations(|m| {
		// This will make any previous inner changes to the element obsolete, so
		// they can be filtered
		m.retain(|m| {
			m.id != parent_id ||
				match m.data {
					MutationData::set_attr(_, _) |
					MutationData::remove_attr(_) => true,
					_ => false,
				}
		});

		m.push(Mutation {
			id: String::from(parent_id),
			data: MutationData::set_inner_html(String::from(html)),
		});
	});
}

// TODO: Make this different from the above, once we have attribute mutators
pub fn set_outer_html(parent_id: &str, html: &str) {
	with_mutations(|m| {
		m.retain(|m| m.id != parent_id);
		m.push(Mutation {
			id: String::from(parent_id),
			data: MutationData::set_outer_html(String::from(html)),
		})
	});
}

macro_rules! define_mutators {
	( $( $id:ident ),* ) => (
		$(
			pub fn $id(parent_id: &str, html: &str) {
				push_mutation(parent_id, MutationData::$id(String::from(html)));
			}
		)*
	)
}

define_mutators!(append, prepend, before, after, remove_attr);

// Remove a node by ID
pub fn remove(id: &str) {
	with_mutations(|m| {
		m.retain(|m| m.id != id);
		m.push(Mutation {
			id: String::from(id),
			data: MutationData::remove,
		});
	});
}

// Push mutation to the stack to be executed on RAF
fn push_mutation(id: &str, data: MutationData) {
	with_mutations(|m| {
		m.push(Mutation {
			id: String::from(id),
			data: data,
		});
	});
}

fn with_mutations<F>(func: F)
where
	F: FnOnce(&mut Vec<Mutation>),
{
	MUTATIONS.with(|r| func(r.borrow_mut().borrow_mut()));
}

// Applies any buffered DOM mutations.
// This is registered to emscripten_set_main_loop by start().
// If you wish to use a different function for the main loop, call this in
// emscripten_set_main_loop with `fps = 0`.
pub extern "C" fn flush_mutations() {
	with_mutations(|mutations| {
		for mutation in mutations.iter() {
			let id = &mutation.id;
			match mutation.data {
				MutationData::append(ref html) => externs::append(id, html),
				MutationData::prepend(ref html) => externs::prepend(id, html),
				MutationData::before(ref html) => externs::before(id, html),
				MutationData::after(ref html) => externs::after(id, html),
				MutationData::set_inner_html(ref html) => {
					externs::set_inner_html(id, html)
				}
				MutationData::set_outer_html(ref html) => {
					externs::set_outer_html(id, html)
				}
				MutationData::remove => externs::remove(id),
				MutationData::set_attr(ref k, ref v) => {
					externs::set_attr(id, k, v)
				}
				MutationData::remove_attr(ref k) => externs::remove_attr(id, k),
			};
		}
		mutations.truncate(0);
	});
}

mod externs {
	use ffi::{from_owned_string, to_borrowed_string};

	// Define functions for writing to the DOM
	macro_rules! define_writers {
		( $( $id:ident ),* ) => (
			$(
				pub fn $id(id: &str, html: &str) {
					unsafe {
						ffi::$id(
							to_borrowed_string(id),
							to_borrowed_string(html),
						)
					};
				}
			)*
		)
	}

	define_writers!(
		set_outer_html,
		set_inner_html,
		append,
		prepend,
		before,
		after
	);

	pub fn remove(id: &str) {
		unsafe { ffi::remove(to_borrowed_string(id)) };
	}

	pub fn remove_attr(id: &str, key: &str) {
		unsafe {
			ffi::remove_attr(to_borrowed_string(id), to_borrowed_string(key))
		};
	}

	pub fn set_attr(id: &str, key: &str, val: &Option<String>) {
		let _val = match *val {
			Some(ref v) => v,
			None => "",
		};
		unsafe {
			ffi::set_attr(
				to_borrowed_string(id),
				to_borrowed_string(key),
				to_borrowed_string(_val),
			)
		};
	}

	// Returns the inner HTML of an element by ID.
	// If no element found, an empty String is returned.
	// Usage of this function will cause extra repaints, so use sparingly.
	pub fn get_inner_html(id: &str) -> String {
		let s = unsafe { ffi::get_inner_html(to_borrowed_string(id)) };
		from_owned_string(s)
	}

	mod ffi {
		use std::os::raw::c_char;

		// Define external functions for writing to the DOM
		macro_rules! define_writers {
			( $( $id:ident ),* ) => (
				extern "C" {
					$( pub fn $id(id: *const c_char, html: *const c_char); )*
				}
			)
		}

		define_writers!(
			set_outer_html,
			set_inner_html,
			append,
			prepend,
			before,
			after
		);

		extern "C" {
			pub fn remove(id: *const c_char);
			pub fn remove_attr(id: *const c_char, key: *const c_char);
			pub fn set_attr(
				id: *const c_char,
				key: *const c_char,
				val: *const c_char,
			);
			pub fn get_inner_html(id: *const c_char) -> *mut c_char;
		}
	}
}
