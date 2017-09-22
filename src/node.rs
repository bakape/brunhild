use mutations::{append, remove, set_outer_html};
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
#[derive(Clone, Default)]
pub struct Node {
	// HTML tag of the node
	pub tag: String,

	// Omitting the value, will produce an attribute with no value
	pub attrs: Attrs,

	// Child Nodes
	pub children: Vec<Node>,
}

impl Node {
	// Renders Node and subtree to HTML.
	// You can persist Saved and later use the diff() method to update the DOM
	// patch the created DOM subtree.
	pub fn render(&self) -> (String, Saved) {
		let mut w = String::with_capacity(1 << 10);
		let saved = Saved::new(self);
		saved.render(&mut w);
		return (w, saved);
	}
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
		if self.tag == "_text" {
			let b = self.attrs.get("_text").unwrap();
			return w.push_str(b.clone().unwrap().as_str());
		}

		write!(w, "<{} id=\"bh-{}\"", self.tag, self.id).unwrap();
		for (ref key, val) in self.attrs.iter() {
			write!(w, " {}", key).unwrap();
			if let &Some(ref val) = val {
				write!(w, "=\"{}\"", &val).unwrap();
			}
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

		self.diff_attrs(&node.attrs);
		self.diff_children(&node.children);
	}

	fn diff_attrs(&mut self, attrs: &Attrs) {
		if self.attrs == *attrs {
			return;
		}

		// TODO: Diff and apply new arguments to element

		self.attrs = attrs.clone();
	}

	fn diff_children(&mut self, children: &[Node]) {
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
