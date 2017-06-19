use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

static mut ID_COUNTER: u64 = 0;

// Generate a new unique view ID
pub fn new_ID() -> String {
	unsafe { ID_COUNTER += 1 };
	format!("brunhild-{}", unsafe { ID_COUNTER })
}

pub enum View<'a> {
	HTML(&'a HTMLView<'a>),
	Parent(&'a ParentView<'a>),
}

macro_rules! base_method {
	($view:expr, $method:ident) => (
		match *$view {
			View::HTML(v) => v.$method(),
			View::Parent(v) => v.$method(),
		}
	)
}

// Should not contain "id"
pub type Attributes = BTreeMap<String, Option<String>>;

pub trait BaseView<'a> {
	fn tag(&self) -> &'a str;
	fn id(&self) -> Option<&'a str>;
	fn attrs(&self) -> Attributes;
	fn state(&self) -> u64;
}

pub trait HTMLView<'a>: BaseView<'a> {
	fn render(&self) -> String;
}

pub trait ParentView<'a>: BaseView<'a> {
	fn children(&self) -> &'a [View];
}

struct Node {
	state: u64,
	tag: String,
	id: String,
	attrs: Attributes,
	children: Vec<Node>,
}

pub struct Tree<'a> {
	view: Rc<RefCell<View<'a>>>,
	node: Node,
}

impl<'a> Tree<'a> {
	fn new(parentID: &str, v: Rc<RefCell<View<'a>>>) -> Tree<'a> {
		// TODO: Insert into DOM
		// TODO: Register render function with RAF

		Tree {
			view: v.clone(),
			node: Node::new(&v.borrow()),
		}
	}

	fn diff(&mut self) {
		diff_node(&mut self.node, &self.view.borrow())
	}
}

impl Node {
	fn new(v: &View) -> Node {
		Node {
			id: match base_method!(v, id) {
				Some(id) => String::from(id),
				None => new_ID(),
			},
			state: base_method!(v, state),
			tag: String::from(base_method!(v, tag)),
			attrs: base_method!(v, attrs),
			children: match *v {
				View::Parent(v) => {
					v.children().iter().map(|ch| Node::new(ch)).collect()
				}
				_ => Vec::new(),
			},
		}
	}
}

fn diff_node(n: &mut Node, v: &View) {
	if base_method!(v, tag) != n.tag {
		return replace_node(n, v);
	}
	if let Some(id) = base_method!(v, id) {
		if id != n.id {
			return replace_node(n, v);
		}
	}

	match *v {
		View::HTML(v) => {
			if v.state() != n.state {
				patch_attrs(n, v.attrs());
				// TODO: Rerender contents
			}
		}
		View::Parent(v) => {
			if v.state() != n.state {
				patch_attrs(n, v.attrs());
			}
			diff_children(n, v.children())
		}
	}
}

fn replace_node(n: &mut Node, v: &View) {
	let old_ID = n.id.clone();
	*n = Node::new(v);
	// TODO: Replace element
}

fn patch_attrs(n: &mut Node, attrs: Attributes) {
	if n.attrs == attrs {
		return;
	}

	// TODO: Diff and apply new arguments to element

	for (key, _) in &attrs {
		assert!(key == "id", "attribute has 'id' key");
	}
	n.attrs = attrs;
}

fn diff_children(parent: &mut Node, views: &[View]) {
	if parent.children.len() == views.len() {
		for (ref mut n, v) in parent.children.iter_mut().zip(views.iter()) {
			diff_node(n, v);
		}
		return;
	}

	if views.len() > parent.children.len() {
		// TODO: Append more children and diff
	}

	// TODO: Rebuild all children
}
