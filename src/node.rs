use mutations::{append, remove, remove_attr, set_attr, set_outer_html};
use std::collections::BTreeMap;
use std::fmt::Write;

static mut ID_COUNTER: u64 = 0;

// Generate a new unique node ID
pub fn new_id() -> String {
	let s = format!("bh-{}", unsafe { ID_COUNTER });
	unsafe { ID_COUNTER += 1 };
	s
}

// Node attributes
pub type Attrs = BTreeMap<String, Option<String>>;

// Represents an HTML Node
#[derive(Default)]
pub struct Node {
	// HTML tag of the node
	pub tag: String,

	// Omitting the value, will produce an attribute with no value
	pub attrs: Attrs,

	// Child Nodes
	pub children: Vec<Node>,
}

// Detect and render a text node as a special case
macro_rules! render_text {
	($self:expr, $w:expr) => (
		if $self.tag == "_text" {
			match $self.attrs.get("_text") {
				Some(t) => {
					match *t {
						Some(ref t) => return $w.push_str(t),
						_ => (),
					}
				}
				_ => (),
			}
		}
	)
}

impl Node {
	// Renders Node and subtree to HTML
	pub fn render(&self) -> String {
		let mut w = String::with_capacity(1 << 10);
		self.write_html(&mut w);
		return w;
	}

	fn write_html(&self, w: &mut String) {
		render_text!(self, w);
		render_start(&self.tag, &self.attrs, w);
		w.push('>');

		for n in self.children.iter() {
			n.write_html(w);
		}

		write!(w, "</{}>", self.tag).unwrap();
	}

	// Renders Node and subtree to HTML.
	// You can persist Saved and later use the patch() method to patch the
	// created DOM subtree.
	pub fn render_virtual(&self) -> (String, Saved) {
		let mut w = String::with_capacity(1 << 10);
		let saved = Saved::new(self);
		saved.render(&mut w);
		return (w, saved);
	}
}

// Renders the start of the element.
// Returns, if one of the attributes was an "id".
fn render_start(tag: &str, attrs: &Attrs, w: &mut String) -> bool {
	write!(w, "<{}", tag).unwrap();

	let mut has_id = false;
	for (ref key, val) in attrs.iter() {
		if *key == "id" {
			has_id = true;
		}
		write!(w, " {}", key).unwrap();
		if let &Some(ref val) = val {
			write!(w, "=\"{}\"", &val).unwrap();
		}
	}
	has_id
}

// Contains a node already rendered in the DOM. Used for persisting the state
// of virtual subtrees.
pub struct Saved {
	id: String,
	tag: String,
	attrs: Attrs,
	children: Vec<Saved>,
}

impl Saved {
	fn new(node: &Node) -> Self {
		Saved {
			// If the element has an id attribute, use it.
			id: match node.attrs.get("id") {
				Some(id) => {
					match *id {
						Some(ref id) => id.clone(),
						None => new_id(),
					}
				}
				None => new_id(),
			},
			tag: node.tag.clone(),
			attrs: node.attrs.clone(),
			children: node.children.iter().map(|n| Saved::new(n)).collect(),
		}
	}

	// Write the Node and its subtree as HTML
	fn render(&self, w: &mut String) {
		render_text!(self, w);
		if !render_start(&self.tag, &self.attrs, w) {
			write!(w, " id=\"{}\"", self.id).unwrap();
		}
		w.push('>');

		for n in self.children.iter() {
			n.render(w);
		}

		write!(w, "</{}>", self.tag).unwrap();
	}

	// Diff Node against the last state of the DOM and apply changes, if any
	pub fn patch(&mut self, node: &Node) {
		// Completely replace node and subtree
		let replace = self.tag != node.tag ||
			match node.attrs.get("id") {
				Some(id) => {
					match *id {
						Some(ref id) => self.id != *id,
						None => false,
					}
				}
				None => false,
			};
		if replace {
			let mut w = String::with_capacity(1 << 10);
			let old_id = self.id.clone();
			*self = Self::new(node);
			self.render(&mut w);
			set_outer_html(&old_id, &w);
			return;
		}

		self.patch_attrs(&node.attrs);
		self.patch_children(&node.children);
	}

	pub fn patch_attrs(&mut self, attrs: &Attrs) {
		if self.attrs == *attrs {
			return;
		}

		// Attributes added or changed
		for (k, v) in attrs.iter() {
			let need_set = match self.attrs.get(k) {
				Some(original_v) => original_v != v,
				None => true,
			};
			if need_set {
				set_attr(
					&self.id,
					k,
					match *v {
						Some(ref v) => Some(&v),
						None => None,
					},
				);
				self.attrs.insert(k.clone(), v.clone());
			}
		}

		// Attributes removed
		let mut to_remove = Vec::<String>::new();
		for (k, _) in self.attrs.iter_mut() {
			match attrs.get(k) {
				Some(_) => (),
				None => {
					remove_attr(&self.id, &k);
					to_remove.push(k.clone());
				}
			};
		}
		for k in to_remove.iter() {
			self.attrs.remove(k);
		}
	}

	fn patch_children(&mut self, children: &[Node]) {
		let mut diff = (children.len() as i32) - (self.children.len() as i32);

		// Remove Nodes from the end
		while diff < 0 {
			remove(&self.children.pop().unwrap().id);
			diff += 1;
		}

		for (ref mut saved, ref ch) in
			self.children.iter_mut().zip(children.iter())
		{
			saved.patch(ch);
		}

		// Append Nodes
		if diff > 0 {
			let mut w = String::with_capacity(1 << 10);
			for ch in children.iter().skip(self.children.len()) {
				w.truncate(0);
				let new = Saved::new(ch);
				new.render(&mut w);
				self.children.push(new);
				append(&self.id, &w);
			}
		}
	}
}
