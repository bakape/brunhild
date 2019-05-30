use super::attrs::Attrs;
use super::classes;
use super::tokenizer;
use super::util;
use std::fmt;
use std::iter::FromIterator;

const IS_TEXT: u8 = 1; // Text Node
const IMMUTABLE: u8 = 1 << 1; // Node contents never change

/*
Node used for constructing DOM trees for applying patches.

This node type does not contain any binding to existing nodes in the DOM tree
or in the pending patches tree. Such relation is determined during diffing.
*/
pub struct Node {
	flags: u8,

	// ID of element the node is representing. This is always zero in
	// user-created nodes and is only set, when a node has been diffed and
	// patched into the DOM representation tree.
	id: u16,

	tag: u16,
	class_set: u16,

	// Inner text content for text nodes
	inner_text: String,

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
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		Self {
			tag: tokenizer::tokenize(tag),
			class_set: super::classes::tokenize(classes),
			attrs: Attrs::from_iter(attrs),
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
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		Self {
			tag: tokenizer::tokenize(tag),
			class_set: super::classes::tokenize(classes),
			attrs: Attrs::from_iter(attrs),
			children: children,
			..Default::default()
		}
	}

	// Create a text node with set inner content.
	// The inner text is HTML-escaped on creation.
	fn text(text: &str) -> Self {
		Self {
			flags: IS_TEXT,
			tag: 0,
			attrs: Attrs::from_iter(
				[("_text", util::html_escape(text).as_str())].iter(),
			),
			..Default::default()
		}
	}

	// Create a text node with set inner content.
	// The inner text is not HTML-escaped on creation.
	fn text_unescaped(text: &str) -> Self {
		Self {
			flags: IS_TEXT,
			tag: 0,
			attrs: Attrs::from_iter([("_text", text)].iter()),
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
}

impl Default for Node {
	fn default() -> Self {
		Self {
			flags: 0,
			id: 0,
			tag: tokenizer::tokenize("div"),
			class_set: 0,
			attrs: Default::default(),
			children: Default::default(),
			inner_text: Default::default(),
		}
	}
}

impl super::WriteHTMLTo for Node {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		let is_text = self.flags & IS_TEXT != 0;

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
