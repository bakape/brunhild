use super::attrs::Attrs;
use super::classes;
use super::tokenizer;
use super::util;
use std::collections::HashMap;
use std::fmt;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

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

// Options for contructing an element Node. This struct has separate lifetimes
// for each field, so that some of these can have static lifeltimes and thus not
// require runtime allocation.
pub struct ElementOptions<'t, 'c, 'a> {
	// Mark element as immutable
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

// Options for contructing an element Node from the JS API.
// These require copying, to avoid lifetimes parameters.
#[derive(Deserialize)]
#[serde(default)]
struct JSElementOptions {
	// Mark element as immutable
	pub immutable: bool,

	// Escape text content, if text node. Defaults to true.
	pub escape: bool,

	// Inner text content for text nodes.
	// If set, all element fields are ignored.
	pub text: String,

	// Element HTML tag. Defaults to "div".
	pub tag: String,

	// Set of classes for the element "class" attribute
	pub classes: Vec<String>,

	// List of element attributes
	pub attributes: HashMap<String, String>,

	// Child Nodes of element
	pub children: Vec<JSElementOptions>,
}

impl Default for JSElementOptions {
	fn default() -> Self {
		Self {
			immutable: false,
			escape: true,
			text: Default::default(),
			tag: "div".into(),
			classes: Default::default(),
			attributes: Default::default(),
			children: Default::default(),
		}
	}
}

#[wasm_bindgen]
impl Node {
	// Create an Element Node from JS
	#[wasm_bindgen(constructor)]
	pub fn js_new_node(opts: &JsValue) -> Node {
		fn new(opts: JSElementOptions) -> Node {
			if opts.text != "" {
				Node::text(&opts.text, opts.escape)
			} else {
				Node::with_children(
					&ElementOptions {
						immutable: opts.immutable,
						tag: &opts.tag,
						classes: opts
							.classes
							.iter()
							.map(|x| x.as_str())
							.collect::<Vec<&str>>()
							.as_slice(),
						attributes: opts
							.attributes
							.iter()
							.map(|x| (x.0.as_str(), x.1.as_str()))
							.collect::<Vec<(&str, &str)>>()
							.iter()
							.collect::<Vec<&(&str, &str)>>()
							.as_slice(),
					},
					opts.children.into_iter().map(new).collect(),
				)
			}
		};

		new(opts.into_serde().unwrap())
	}

	// Mark node and its contents as immutable. They will never be diffed or
	// patched. The node will still be replaced, if its parent node is replaced.
	#[inline]
	pub fn mark_immutable(&mut self) {
		self.flags |= IMMUTABLE;
	}
}

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"

export type NodeOptions = {
	// Mark element as immutable
	immutable?: boolean;

	// Escape text content, if text node. Defaults to true.
	escape?: boolean;

	// Inner text content for text nodes.
	// If set, all element fields are ignored.
	text?: string;

	// Element HTML tag. Defaults to "div".
	tag?: string;

	// Set of classes for the element "class" attribute
	classes?: string;

	// List of element attributes
	attributes?: {[key: string]: string};

	// Child Nodes of element
	children?: NodeOptions[];
};

// TODO: override autogenerated constructor type signature
// TODO: Add docs to TS definitions

"#;

impl Node {
	// Create an Element Node
	pub fn element(opts: &ElementOptions) -> Self {
		Self {
			contents: NodeContents::Element(ElementContents {
				common: ElementContentsCommon {
					tag: tokenizer::tokenize(opts.tag),
					class_set: super::classes::tokenize(opts.classes),
					attrs: super::attrs::Attrs::new(opts.attributes),
				},
				..Default::default()
			}),
			..Default::default()
		}
	}

	// Create an Element Node with children
	pub fn with_children(opts: &ElementOptions, children: Vec<Node>) -> Self {
		Self {
			contents: NodeContents::Element(ElementContents {
				common: ElementContentsCommon {
					tag: tokenizer::tokenize(opts.tag),
					class_set: super::classes::tokenize(opts.classes),
					attrs: super::attrs::Attrs::new(opts.attributes),
				},
				children: children,
			}),
			..Default::default()
		}
	}

	// Create a text node with set inner content
	//
	// escape: optional HTML escaping
	pub fn text(text: &str, escape: bool) -> Self {
		Self {
			contents: NodeContents::Text(if escape {
				util::html_escape(text.into())
			} else {
				text.into()
			}),
			..Default::default()
		}
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
	let node = Node::text("<span>", true);
	match node.contents {
		NodeContents::Text(t) => assert_eq!(&t, "&lt;span&gt;"),
		_ => assert!(false),
	};
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
