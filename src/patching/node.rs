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
	#[inline]
	fn new(tag: &str) -> Self {
		Self {
			tag: tokenizer::tokenize(tag),
			..Default::default()
		}
	}

	// Create a node with predefined class list
	#[inline]
	fn with_classes<'a, C>(tag: &str, classes: C) -> Self
	where
		C: IntoIterator<Item = &'a str>,
	{
		let mut s = Self::new(tag);
		s.class_set = super::classes::tokenize(classes);
		s
	}

	// Create a node with a predefined class list and attribute map
	#[inline]
	fn with_attrs<'a, 'b, C, A>(tag: &str, classes: C, attrs: A) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = (&'b str, &'b str)>,
	{
		let mut s = Self::with_classes(tag, classes);
		s.attrs = Attrs::new(attrs);
		s
	}

	// Create a node with a predefined class list, attribute map and child list
	#[inline]
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
		let mut s = Self::with_attrs(tag, classes, attrs);
		s.children = children;
		s
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
