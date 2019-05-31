use super::attrs::Attrs;
use super::classes;
use super::tokenizer;
use super::util;
use std::fmt;
use std::iter::FromIterator;

const TEXT_NODE: u8 = 1; // Text Node
const IMMUTABLE: u8 = 1 << 1; // Node contents never change
const DIRTY: u8 = 1 << 2; // Node contents not synced to DOM yet

/*
Node used for constructing DOM trees for applying patches.

This node type does not contain any binding to existing nodes in the DOM tree
or in the pending patches tree. Such relation is determined during diffing.
*/
pub struct Node {
	flags: u8,

	tag: u16,
	class_set: u16,

	// Inner text content for text nodes
	inner_text: String,

	// Node attributes, excluding "id" and "class".
	// "id" is used internally for node addresing and can not be set.
	// to set "class" used the dedicated methods.
	pub attrs: Attrs,

	// Children of Node
	pub children: Vec<Node>,
}

impl Node {
	// Create new node with only the tag field set
	#[inline]
	pub fn new(tag: &str) -> Self {
		Self {
			tag: tokenizer::tokenize(tag),
			..Default::default()
		}
	}

	// Create a node with predefined class list
	#[inline]
	pub fn with_classes<'a, C>(tag: &str, classes: C) -> Self
	where
		C: IntoIterator<Item = &'a str>,
	{
		let mut s = Self::new(tag);
		s.class_set = super::classes::tokenize(classes);
		s
	}

	// Create a node with a predefined class list and attribute map
	#[inline]
	pub fn with_attrs<'a, 'b, C, A>(tag: &str, classes: C, attrs: A) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		let mut s = Self::with_classes(tag, classes);
		s.attrs = Attrs::from_iter(attrs);
		s
	}

	// Create a node with a predefined class list, attribute map and child list
	#[inline]
	pub fn with_children<'a, 'b, C, A>(
		tag: &str,
		classes: C,
		attrs: A,
		children: Vec<Node>,
	) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		let mut s = Self::with_attrs(tag, classes, attrs);
		s.children = children;
		s
	}

	// Create a text node with set inner content.
	// The inner text is HTML-escaped on creation.
	#[inline]
	pub fn text<'a, T: Into<&'a str>>(text: T) -> Self {
		Self::text_unescaped(util::html_escape(text.into()))
	}

	// Create a text node with set inner content.
	// The inner text is not HTML-escaped on creation.
	#[inline]
	pub fn text_unescaped<T: Into<String>>(text: T) -> Self {
		Self {
			flags: TEXT_NODE | DIRTY,
			tag: 0,
			inner_text: text.into(),
			..Default::default()
		}
	}

	// Set HTML tag of node
	pub fn set_tag(&mut self, tag: &str) {
		self.tag = tokenizer::tokenize(tag);
	}

	// Add class to Node class set
	pub fn add_class(&mut self, class: &str) {
		classes::add_class(&mut self.class_set, class);
	}

	// Remove class from Node class set
	pub fn remove_class(&mut self, class: &str) {
		classes::remove_class(&mut self.class_set, class);
	}

	// Mark node and its contents as immutable. They will never be diffed or
	// patched. The node will still be replaced, if its parent node is replaced.
	#[inline]
	pub fn mark_immutable(&mut self) {
		self.flags |= IMMUTABLE;
	}

	// Return, if node is marked immutable
	#[inline]
	fn is_immutable(&self) -> bool {
		self.flags & IMMUTABLE != 0
	}

	// Merge a possibly changed version of Self into self for patching the
	// pending change tree
	fn merge(&mut self, mut new: Self) {
		if !self.is_immutable() {
			*self = new;
		}
	}
}

impl Default for Node {
	fn default() -> Self {
		Self {
			flags: DIRTY,
			tag: tokenizer::tokenize("div"),
			class_set: 0,
			attrs: Default::default(),
			children: Default::default(),
			inner_text: Default::default(),
		}
	}
}

// Node mapped to an element exisitng in the DOM tree
pub struct DOMNode {
	flags: u8,

	// ID of element the node is representing.
	id: u16,

	tag: u16,
	class_set: u16,

	// Inner text content for text nodes
	inner_text: String,

	// Node attributes, excluding "id" and "class".
	// "id" is used internally for node addresing and can not be set.
	// to set "class" used the dedicated methods.
	pub attrs: Attrs,

	// Children of Node
	pub children: Vec<DOMNode>,
}

impl DOMNode {
	// Return, if node is a text node
	#[inline]
	fn is_text(&self) -> bool {
		self.flags & TEXT_NODE != 0
	}

	// Return, if node is marked immutable
	#[inline]
	fn is_immutable(&self) -> bool {
		self.flags & IMMUTABLE != 0
	}
}

impl super::WriteHTMLTo for DOMNode {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		let is_text = self.is_text();

		macro_rules! write_tag {
			() => {
				if is_text {
					w.write_str("span")?;
				} else {
					tokenizer::write_html_to(self.tag, w)?;
					}
			};
		}

		w.write_char('<')?;
		write_tag!();
		write!(w, " id=\"bh-{}\"", self.id)?;
		if !is_text {
			if self.class_set != 0 {
				w.write_str(" class=")?;
				classes::write_html_to(self.class_set, w)?;
				w.write_char('"')?;
			}
			self.attrs.write_html_to(w)?;
		}
		w.write_char('>')?;

		if is_text {
			w.write_str(&self.inner_text)?;
		} else {
			match self.tag {
				// <br>, <hr> and <wbr> must not be closed.
				// Some browsers will interpret that as 2 tags.
				36 | 124 | 282 => {
					return Ok(());
				}
				_ => {
					for ch in self.children.iter() {
						ch.write_html_to(w)?;
					}
				}
			};
		}

		w.write_str("</")?;
		write_tag!();
		w.write_char('>')
	}
}
