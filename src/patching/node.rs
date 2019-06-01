use super::attrs::Attrs;
use super::classes;
use super::patching;
use super::tokenizer;
use super::util;
use std::fmt;
use std::rc::Rc;

const IMMUTABLE: u8 = 1; // Node contents never change
const DIRTY: u8 = 1 << 1; // Node contents not synced to DOM yet

// Internal contents of a text Node or Element
#[derive(Clone)]
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
#[derive(Clone)]
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
#[derive(Default, Clone)]
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

// Node used for constructing DOM trees for applying patches.
//
// This node type does not contain any binding to existing nodes in the DOM tree
// or in the pending patches tree. Such relation is determined during diffing.
#[derive(Clone)]
pub struct Node {
	flags: u8,

	// Kee used to identify the same node, during potentially destructive
	// patching. Only set, if this node requires persistance, like maintaining
	// user input focus or selections.
	key: Option<u64>,

	// Handle assigned to this node
	handle: Option<Rc<Handle>>,

	contents: NodeContents,
}

impl Default for Node {
	fn default() -> Self {
		Self {
			flags: DIRTY,
			key: None,
			handle: None,
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

	// Kee used to identify the same node, during potentially destructive
	// patching. Only set, if this node requires persistance, like maintaining
	// user input focus or selections.
	pub key: Option<u64>,

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
			key: None,
			classes: &[],
			attributes: &[],
		}
	}
}

// Options for constructing a text Node
pub struct TextOptions<'a> {
	// Mark node as immutable
	pub immutable: bool,

	// HTML-escape inner text
	pub escape: bool,

	// Element text content
	pub text: &'a str,

	// Kee used to identify the same node, during potentially destructive
	// patching. Only set, if this node requires persistance, like maintaining
	// user input focus or selections.
	pub key: Option<u64>,
}

impl<'a> Default for TextOptions<'a> {
	fn default() -> Self {
		Self {
			immutable: false,
			escape: true,
			text: "",
			key: None,
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
		if self.is_immutable() {
			return;
		}

		// Completely replace node and subtree
		let replace = match &self.key {
			Some(k) => match &new.key {
				Some(new_k) => new_k != k,
				None => true,
			},
			None => match &new.key {
				Some(_) => true,
				None => false,
			},
		} || match &mut self.contents {
			NodeContents::Text(ref mut text) => match &mut new.contents {
				NodeContents::Element(_) => true,
				NodeContents::Text(new_text) => {
					std::mem::swap(text, new_text);
					false
				}
			},
			NodeContents::Element(ref mut cont) => match &mut new.contents {
				NodeContents::Text(_) => true,
				NodeContents::Element(ref mut new_cont) => {
					if new_cont.common.tag != cont.common.tag {
						true
					} else {
						std::mem::swap(
							&mut cont.common.attrs,
							&mut new_cont.common.attrs,
						);
						cont.common.class_set = new_cont.common.class_set;

						// TODO: Patch children

						false
					}
				}
			},
		};
		if replace {
			*self = new;
			return;
		}

		// TODO: When patching, being dirty should take priority over being
		// immutable, in case immutability was added later. Explain this in a
		// comment.

		// The entire subtree will be marked as dirty with this.
		// Completely replaced nodes already have the right flags.
		self.flags |= DIRTY;
		if new.is_immutable() {
			self.flags |= IMMUTABLE;
		}

		// Update handle, in case it changed, to keep the pointers equal
		self.handle = new.handle;
	}

	// Merge a possibly changed child subtree for patching the pending change
	// tree
	fn merge_children(&mut self, new: &mut Vec<Node>) {
		unimplemented!()
	}

	// Take a handle for Node to allow performing actions on it after it has
	// been merged into the DOM tree.
	//
	// Note, that destructively patching the parent tree causes the handle to
	// become dangling and NOP on all further operations.
	// Destructive parent patches can include:
	// 	- Changing the tag of on a element node
	// 	- Changing the key of a node
	// 	- Removing a node
	// 	- Changing the order of exiting child nodes in a list of child nodes, if
	// 	  the nodes do not have the "key" property set to prevent such
	// 	  destructive changes.
	// It is the responsibility of the library user to be aware of this handle
	// invalidation and not destructively path the parent subtree, when not
	// needed.
	pub fn take_handle(&mut self) -> Rc<Handle> {
		let h = Rc::new(Handle::default());
		self.handle = Some(h.clone());
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

	// ID of element the node is representing.
	id: u64,

	// Kee used to identify the same node, during potentially destructive
	// patching. Only set, if this node requires persistance, like maintaining
	// user input focus or selections.
	key: Option<u64>,

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
			key: None,
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

// Provides methods for manipulating a Node and its subtree
#[derive(Default)]
pub struct Handle {}

impl Handle {
	// Queue pending patches for handled node and its subtree.
	// Returns, if the handled node has been found in the pending change tree.
	pub fn patch(&mut self, new: Node) -> bool {
		let found = self.lookup_pending(|old| {
			old.merge(new);
		});
		if found {
			unsafe { patching::DIRTY = true };
		}
		return found;
	}

	// Lookup node inside the tree using a hybrid breadth-then-depth first algo.
	//
	// Returns, if a node was found.
	fn lookup_pending<F>(&mut self, func: F) -> bool
	where
		F: FnOnce(&mut Node),
	{
		// TODO: Lookup cache (vector of parent IDs)

		util::with_global(&patching::PENDING, |r| {
			if self.same_handle(r) {
				// Root is the target node
				func(r);
				true
			} else if let Some(n) = self.traverse_pending(r) {
				func(n);
				true
			} else {
				false
			}
		})
	}

	// Compare handles using pointer equality
	fn same_handle(&self, n: &Node) -> bool {
		match &n.handle {
			Some(h) => &**h as *const Handle == self as *const Handle,
			None => false,
		}
	}

	fn traverse_pending<'a: 'b, 'b>(
		&mut self,
		n: &'a mut Node,
	) -> Option<&'b mut Node> {
		match &mut n.contents {
			NodeContents::Text(_) => None,
			NodeContents::Element(ref mut cont) => {
				// First search the direct children of the node. This vector
				// scan is cheaper on cache locality and reduces the chance
				// of needlessly going too deep, as handles should typically
				// be not at tree bottom.
				//
				// Using .position() instead of .find() to "let go" of reference
				// to cont.children and make Rust not see the None case as
				// somehow taking a mutable reference of cont.children
				// concurrently with the Some() case.
				//
				// Dumb borrow checker.
				match cont
					.children
					.iter_mut()
					.position(|ch| self.same_handle(ch))
				{
					Some(i) => Some(&mut cont.children[i]),
					None => {
						// If not found, go a level deeper on each node
						match cont
							.children
							.iter_mut()
							.find_map(|ch| self.traverse_pending(ch))
						{
							Some(ch) => Some(ch),
							None => None,
						}
					}
				}
			}
		}
	}

	// TODO: Event handling
}
