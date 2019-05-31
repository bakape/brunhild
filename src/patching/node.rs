use super::attrs::Attrs;
use super::classes;
use super::tokenizer;
use super::util;
use std::fmt;
use std::iter::FromIterator;

const IMMUTABLE: u8 = 1; // Node contents never change
const DIRTY: u8 = 1 << 1; // Node contents not synced to DOM yet

// Internal contents of a text Node or Element
enum NodeContents {
	Text(String),
	Element(ElementContents),
}

impl Default for NodeContents {
	fn default() -> Self {
		NodeContents::Element(Default::default())
	}
}

// Common Element contents between Node and DOMNode
struct ElementContentsCommon {
	// Token for the node's tag
	tag: u16,

	// Token for compressed set of classes for this node
	class_set: u16,

	// Node attributes, excluding "id" and "class".
	// "id" is used internally for node addresing and can not be set.
	// to set "class" used the dedicated methods.
	attrs: Attrs,
}

impl Default for ElementContentsCommon {
	fn default() -> Self {
		Self {
			tag: tokenizer::tokenize("div"),
			class_set: 0,
			attrs: Default::default(),
		}
	}
}

// Internal contents of an Element
#[derive(Default)]
struct ElementContents {
	common: ElementContentsCommon,

	// Children of Node
	children: Vec<Node>,
}

impl ElementContents {
	#[inline]
	fn new(tag: &str) -> Self {
		Self {
			common: ElementContentsCommon {
				tag: tokenizer::tokenize(tag),
				..Default::default()
			},
			..Default::default()
		}
	}

	#[inline]
	fn with_classes<'a, C>(tag: &str, classes: Option<C>) -> Self
	where
		C: IntoIterator<Item = &'a str>,
	{
		let mut s = Self::new(tag);
		if let Some(c) = classes {
			s.common.class_set = super::classes::tokenize(c);
		}
		s
	}

	#[inline]
	fn with_attrs<'a, 'b, C, A>(
		tag: &str,
		classes: Option<C>,
		attrs: Option<A>,
	) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		let mut s = Self::with_classes(tag, classes);
		if let Some(a) = attrs {
			s.common.attrs = Attrs::from_iter(a);
		}
		s
	}

	#[inline]
	fn with_children<'a, 'b, C, A>(
		tag: &str,
		classes: Option<C>,
		attrs: Option<A>,
		children: Option<Vec<Node>>,
	) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		let mut s = Self::with_attrs(tag, classes, attrs);
		if let Some(c) = children {
			s.children = c;
		}
		s
	}
}

// Internal contents of a text Node or Element in the DOm representation tree
enum DOMNodeContents {
	Text(String),
	Element(DOMElementContents),
}

// Internal contents of an Element in the DOM representation tree
struct DOMElementContents {
	common: ElementContentsCommon,

	// Children of Node
	children: Vec<DOMNode>,
}

/*
Node used for constructing DOM trees for applying patches.

This node type does not contain any binding to existing nodes in the DOM tree
or in the pending patches tree. Such relation is determined during diffing.
*/
pub struct Node {
	flags: u8,
	contents: NodeContents,
}

impl Default for Node {
	fn default() -> Self {
		Self {
			flags: DIRTY,
			contents: Default::default(),
		}
	}
}

impl Node {
	// Create new node with only the tag field set\
	#[inline]
	pub fn new(tag: &str) -> Self {
		Self {
			contents: NodeContents::Element(ElementContents::new(tag)),
			..Default::default()
		}
	}

	// Create a node witha n optional class list
	#[inline]
	pub fn with_classes<'a, C>(tag: &str, classes: Option<C>) -> Self
	where
		C: IntoIterator<Item = &'a str>,
	{
		Self {
			contents: NodeContents::Element(ElementContents::with_classes(
				tag, classes,
			)),
			..Default::default()
		}
	}

	// Create a node with an optional class list and attribute map
	#[inline]
	pub fn with_attrs<'a, 'b, C, A>(
		tag: &str,
		classes: Option<C>,
		attrs: Option<A>,
	) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		Self {
			contents: NodeContents::Element(ElementContents::with_attrs(
				tag, classes, attrs,
			)),
			..Default::default()
		}
	}

	// Create a node with an optional class list, attribute map and child list
	#[inline]
	pub fn with_children<'a, 'b, C, A>(
		tag: &str,
		classes: Option<C>,
		attrs: Option<A>,
		children: Option<Vec<Node>>,
	) -> Self
	where
		C: IntoIterator<Item = &'a str>,
		A: IntoIterator<Item = &'b (&'b str, &'b str)>,
	{
		Self {
			contents: NodeContents::Element(ElementContents::with_children(
				tag, classes, attrs, children,
			)),
			..Default::default()
		}
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
			contents: NodeContents::Text(text.into()),
			..Default::default()
		}
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

// Node mapped to an element exisitng in the DOM tree
pub struct DOMNode {
	flags: u8,
	id: u16, // ID of element the node is representing.
	contents: DOMNodeContents,
}

impl DOMNode {
	// Return, if node is marked immutable
	#[inline]
	fn is_immutable(&self) -> bool {
		self.flags & IMMUTABLE != 0
	}
}

impl super::WriteHTMLTo for DOMNode {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		match &self.contents {
			DOMNodeContents::Text(text) => {
				write!(w, "<span id=\"bh-{}\">{}</span>", self.id, text)
			}
			DOMNodeContents::Element(cont) => {
				w.write_char('<')?;
				tokenizer::write_html_to(cont.common.tag, w)?;
				write!(w, " id=\"bh-{}\"", self.id)?;
				if cont.common.class_set != 0 {
					w.write_str(" class=")?;
					classes::write_html_to(cont.common.class_set, w)?;
					w.write_char('"')?;
				}
				cont.common.attrs.write_html_to(w)?;
				w.write_char('>')?;

				match cont.common.tag {
					// <br>, <hr> and <wbr> must not be closed.
					// Some browsers will interpret that as 2 tags.
					36 | 124 | 282 => {
						return Ok(());
					}
					_ => {
						for ch in cont.children.iter() {
							ch.write_html_to(w)?;
						}
					}
				};

				w.write_str("</")?;
				tokenizer::write_html_to(cont.common.tag, w)?;
				w.write_char('>')
			}
		}
	}
}
