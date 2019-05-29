use super::attrs::Attrs;
use super::classes;
use super::tokenizer;

/*
	Node used for constructing DOM trees for applying patches.

	This node type does not contain any binding to existing nodes in the DOM
	tree or in the pending patches tree. Such relation is determined during
	diffing.
*/
pub struct Node {
	tag: u16,
	class_set: u16,
	pub attrs: Attrs,
	pub children: Vec<Node>,
}

impl Node {
	// Create new node with only the tag field set
	fn new(tag: &str) -> Self {
		Self {
			tag: tokenizer::tokenize(tag),
			..Default::default()
		}
	}

	// Create a node with predefined class list
	fn with_classes<'a, C>(tag: &str, classes: C) -> Self
	where
		C: IntoIterator<Item = &'a str>,
	{
		Self {
			tag: tokenizer::tokenize(tag),
			class_set: super::classes::tokenize(classes),
			..Default::default()
		}
	}

	// Create a node with a predefined class list and attribute map
	fn with_attrs<'a, 'b, C, A>(tag: &str, classes: C, attrs: A) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = (&'b str, &'b str)>,
	{
		Self {
			tag: tokenizer::tokenize(tag),
			class_set: super::classes::tokenize(classes),
			attrs: Attrs::new(attrs),
			..Default::default()
		}
	}

	// Create a node with a predefined class list, attribute map and child list
	fn with_children<'a, 'b, C, A>(
		tag: &str,
		classes: C,
		attrs: A,
		children: Vec<Node>,
	) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = (&'b str, &'b str)>,
	{
		Self {
			tag: tokenizer::tokenize(tag),
			class_set: super::classes::tokenize(classes),
			attrs: Attrs::new(attrs),
			children: children,
			..Default::default()
		}
	}

	// Set HTML tag of node
	pub fn set_tag(&mut self, tag: &str) {
		self.tag = tokenizer::tokenize(tag);
	}

	// Add class to Node class set
	pub fn add_class(&mut self, class: &str) {
		unimplemented!()
	}

	// Remove class from Node class set
	pub fn remove_class(&mut self, class: &str) {
		unimplemented!()
	}
}

impl Default for Node {
	fn default() -> Self {
		Self {
			tag: tokenizer::tokenize("div"),
			class_set: 0,
			attrs: Default::default(),
			children: Default::default(),
		}
	}
}
