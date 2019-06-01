use super::attrs::Attrs;
use super::classes;
use super::patching::Handle;
use super::tokenizer;
use super::util;
use std::fmt;
use std::rc::Rc;
use wasm_bindgen::prelude::wasm_bindgen;

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
#[wasm_bindgen]
pub struct Node {
	flags: u8,

	// Handle pending assignment on the next patch
	pending_handle: Option<Rc<Handle>>,

	contents: NodeContents,
}

impl Default for Node {
	fn default() -> Self {
		Self {
			flags: DIRTY,
			pending_handle: None,
			contents: Default::default(),
		}
	}
}

// Options for constructing an Element Node. This struct has separate lifetimes
// for each field, so that some of these can have static lifetimes and thus not
// require runtime allocation.
pub struct ElementOptions<'t, 'c, 'a> {
	// Mark node as immutable
	pub immutable: bool,

	// Element HTML tag
	pub tag: &'t str,

	// Set of classes for the element "class" attribute
	pub classes: &'c [&'c str],

	// List of element attributes
	pub attributes: &'a [&'a (&'a str, &'a str)],
}

impl<'t, 'c, 'a> Default for ElementOptions<'t, 'c, 'a> {
	fn default() -> Self {
		Self {
			immutable: false,
			tag: "div",
			classes: &[],
			attributes: &[],
		}
	}
}

// Options for contructing a text Node
pub struct TextOptions<'a> {
	// Mark node as immutable
	pub immutable: bool,

	// HTML-escape inner text
	pub escape: bool,

	// Element text content
	pub text: &'a str,
}

impl<'a> Default for TextOptions<'a> {
	fn default() -> Self {
		Self {
			immutable: false,
			escape: true,
			text: "",
		}
	}
}

impl Node {
	// Create an Element Node
	pub fn element(opts: &ElementOptions) -> Self {
		let mut s = Self {
			contents: NodeContents::Element(ElementContents {
				common: ElementContentsCommon {
					tag: tokenizer::tokenize(opts.tag),
					class_set: super::classes::tokenize(opts.classes),
					attrs: super::attrs::Attrs::new(opts.attributes),
				},
				..Default::default()
			}),
			..Default::default()
		};
		if opts.immutable {
			s.flags |= IMMUTABLE;
		}
		s
	}

	// Create an Element Node with children
	pub fn with_children(opts: &ElementOptions, children: Vec<Node>) -> Self {
		let mut s = Self {
			contents: NodeContents::Element(ElementContents {
				common: ElementContentsCommon {
					tag: tokenizer::tokenize(opts.tag),
					class_set: super::classes::tokenize(opts.classes),
					attrs: super::attrs::Attrs::new(opts.attributes),
				},
				children: children,
			}),
			..Default::default()
		};
		if opts.immutable {
			s.flags |= IMMUTABLE;
		}
		s
	}

	// Create a text node with set inner content
	//
	// escape: optional HTML escaping
	pub fn text(opts: &TextOptions) -> Self {
		let mut s = Self {
			contents: NodeContents::Text(if opts.escape {
				util::html_escape(opts.text.into())
			} else {
				opts.text.into()
			}),
			..Default::default()
		};
		if opts.immutable {
			s.flags |= IMMUTABLE;
		}
		s
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

	// Take a handle for Node to allow performing actions on it after it has
	// been merged into the DOM tree
	pub fn take_handle(&mut self) -> Rc<Handle> {
		let h = Rc::new(Handle::default());
		self.pending_handle = Some(h.clone());
		h
	}
}

#[test]
fn create_element_node() {
	#[allow(unused)]
	let node = Node::element(&ElementOptions {
		tag: "span",
		classes: &["class1", "class2"],
		attributes: &[&("disabled", ""), &("width", "64")],
		..Default::default()
	});
}

#[test]
fn create_element_node_with_children() {
	#[allow(unused)]
	let node = Node::with_children(
		&ElementOptions {
			tag: "span",
			classes: &["class1", "class2"],
			attributes: &[&("disabled", ""), &("width", "64")],
			..Default::default()
		},
		vec![Node::element(&ElementOptions {
			tag: "span",
			classes: &["class1", "class2"],
			attributes: &[&("disabled", ""), &("width", "64")],
			..Default::default()
		})],
	);
}

#[test]
fn create_text_node() {
	let node = Node::text(&TextOptions {
		text: "<span>",
		..Default::default()
	});
	match node.contents {
		NodeContents::Text(t) => assert_eq!(&t, "&lt;span&gt;"),
		_ => assert!(false),
	};
}

// Node mapped to an element existing in the DOM tree
pub struct DOMNode {
	flags: u8,
	id: u64, // ID of element the node is representing.
	handle: Option<Rc<Handle>>,
	contents: DOMNodeContents,
}

impl DOMNode {
	// Return, if node is marked immutable
	#[inline]
	fn is_immutable(&self) -> bool {
		self.flags & IMMUTABLE != 0
	}
}

impl Default for DOMNode {
	fn default() -> Self {
		Self {
			flags: 0,
			id: 0,
			handle: None,
			contents: DOMNodeContents::Text(String::new()),
		}
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
