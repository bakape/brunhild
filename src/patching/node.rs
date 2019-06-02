use super::attrs::Attrs;
use super::classes;
use super::patching;
use super::tokenizer;
use super::util;
use std::collections::HashMap;
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

	// ID of DOM element the node is representing. Can be 0 in nodes not yet
	// patched into the DOM.
	id: u64,

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
			id: 0,
			handle: None,
			contents: Default::default(),
		}
	}
}

// Options for constructing an Element Node. This struct has separate lifetimes
// for each field, so that some of these can have static lifetimes and thus not
// require runtime allocation.
pub struct ElementOptions<'t, 'c, 'a> {
	// Mark node and its entire subtree as immutable. Such a node will never be
	// merged or patched and thus can improve performance.
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
	// Mark node and its entire subtree as immutable. Such a node will never be
	// merged or patched and thus can improve performance.
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
	fn merge(&mut self, new: Self) {
		if self.is_immutable() {
			return;
		}
		if !Node::nodes_match(self, &new) {
			// Completely replace node and subtree
			*self = new;
		} else {
			// The entire subtree will be marked as dirty with this.
			// Completely replaced nodes already have the right flags.
			self.flags |= DIRTY;

			Node::merge_node(self, new);
		}
	}

	// Return, if nodes are considered similar enough to be merged and not
	// replaced destructively
	fn nodes_match(old: &Node, new: &Node) -> bool {
		(match old.key {
			Some(k) => match new.key {
				Some(new_k) => new_k == k,
				None => false,
			},
			None => new.key.is_none(),
		} || match &old.contents {
			NodeContents::Text(_) => match &new.contents {
				NodeContents::Element(_) => false,
				NodeContents::Text(_) => true,
			},
			NodeContents::Element(cont) => match &new.contents {
				NodeContents::Text(_) => false,
				NodeContents::Element(new_cont) => {
					new_cont.common.tag == cont.common.tag
				}
			},
		})
	}

	// Merge a matching node new into old. See Node::nodes_match().
	fn merge_node(old: &mut Node, new: Node) {
		// TODO: When patching, being dirty should take priority over being
		// immutable, in case immutability was added later. Explain this in a
		// comment.

		if new.is_immutable() {
			old.flags |= IMMUTABLE;
		}

		// Update handle, in case it changed, to keep the pointers equal
		old.handle = new.handle;

		match &mut old.contents {
			NodeContents::Text(ref mut text) => match new.contents {
				NodeContents::Text(new_text) => {
					*text = new_text;
				}
				_ => (),
			},
			NodeContents::Element(ref mut cont) => match new.contents {
				NodeContents::Element(new_cont) => {
					cont.common.attrs = new_cont.common.attrs;
					cont.common.class_set = new_cont.common.class_set;
					Node::merge_children(&mut cont.children, new_cont.children);
				}
				_ => (),
			},
		}
	}

	// Merge a possibly changed child subtree for patching the pending change
	// tree
	fn merge_children(old: &mut Vec<Node>, mut new: Vec<Node>) {
		let mut old_it = old.iter_mut().peekable();
		let mut new_it = new.into_iter().peekable();
		let mut nodes_mismatched = false;

		// First merge all matching children. Most of the time child lists will
		// match, so this is the hottest loop.
		while old_it.peek().is_some() && new_it.peek().is_some() {
			let old_ch = old_it.next().unwrap();
			let new_ch = new_it.next().unwrap();

			if !Node::nodes_match(old_ch, &new_ch) {
				nodes_mismatched = true;
				break;
			}
			if old_ch.is_immutable() {
				continue;
			}
			Node::merge_node(old_ch, new_ch);
		}

		if nodes_mismatched {
			// Match the rest of the nodes by key, if any
			let i = old_it.count();
			new = new_it.collect();

			// Check we actually have any keys
			let mut have_keys = false;
			for ch in old.iter().skip(i) {
				if ch.key.is_some() {
					have_keys = true;
					break;
				}
			}
			if !have_keys {
				for ch in new.iter() {
					if ch.key.is_some() {
						have_keys = true;
						break;
					}
				}
			}

			if !have_keys {
				// Destructively swap in nodes
				let mut old_it = old.iter_mut().skip(i).peekable();
				new_it = new.into_iter().peekable();
				while old_it.peek().is_some() && new_it.peek().is_some() {
					*old_it.next().unwrap() = new_it.next().unwrap();
				}

				// Handle mismatched node counts using appends or deletes
				if new_it.peek().is_some() {
					// Append new nodes to end
					old.extend(new_it);
				} else if old_it.peek().is_some() {
					// Remove nodes from end
					let left = old_it.count();
					old.truncate(old.len() - left);
				}
			} else {
				// Perform matching by key and destructively swap in the rest
				let mut old_by_key: HashMap<u64, Node> = old
					.split_off(i)
					.into_iter()
					.filter_map(|ch| match ch.key {
						Some(k) => Some((k, ch)),
						None => None,
					})
					.collect();
				for new_ch in new {
					match new_ch.key {
						Some(k) => match old_by_key.remove(&k) {
							Some(mut old_ch) => {
								Node::merge_node(&mut old_ch, new_ch);
								old.push(old_ch);
							}
							None => {
								old.push(new_ch);
							}
						},
						None => {
							old.push(new_ch);
						}
					}
				}
			}
		} else {
			// Handle mismatched node counts using appends or deletes
			if new_it.peek().is_some() {
				// Append new nodes to end
				old.extend(new_it);
			} else if old_it.peek().is_some() {
				// Remove nodes from end
				let left = old_it.count();
				old.truncate(old.len() - left);
			}
		}
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

	// Key used to identify the same node, during potentially destructive
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
pub struct Handle {
	// Last path the node was found at
	lookup_cache: Vec<u64>,
}

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
		util::with_global(&patching::PENDING, |r| {
			// Lookup value by path cache, if any
			match self.lookup_cache.len() {
				0 => {
					// No cache
					self.find_pending_no_cache(r)
				}
				1 => {
					// Root node is cached
					if self.same_handle(r) {
						Some(r)
					} else {
						self.lookup_cache.truncate(0);
						None
					}
				}
				_ => {
					// Child node in cache
					match Handle::find_pending_by_cache(r, &self.lookup_cache) {
						Some(n) => Some(n),
						None => {
							// Cache miss
							self.lookup_cache.truncate(0);
							self.find_pending_no_cache(r)
						}
					}
				}
			}
			.map(|n| func(n))
			.is_some()
		})
	}

	// Compare handles using pointer equality
	fn same_handle(&self, n: &Node) -> bool {
		match &n.handle {
			Some(h) => &**h as *const Handle == self as *const Handle,
			None => false,
		}
	}

	// Attempt to find a node in the pending change tree using the lookup cache
	fn find_pending_by_cache<'a: 'b, 'b>(
		n: &'a mut Node,
		path: &[u64],
	) -> Option<&'b mut Node> {
		match &mut n.contents {
			NodeContents::Text(_) => None,
			NodeContents::Element(cont) => {
				match cont.children.iter_mut().find(|ch| ch.id == path[0]) {
					Some(ch) => Handle::find_pending_by_cache(ch, &path[1..]),
					None => None,
				}
			}
		}
	}

	fn find_pending_no_cache<'a: 'b, 'b>(
		&mut self,
		r: &'a mut Node,
	) -> Option<&'b mut Node> {
		if self.same_handle(r) {
			// Root is the target node
			if r.id != 0 {
				self.lookup_cache.truncate(0);
				self.lookup_cache.push(r.id);
			}
			Some(r)
		} else if let Some(res) = self.traverse_pending(r) {
			match res.1 {
				Some(path) => {
					self.lookup_cache = path;
					self.lookup_cache.reverse();
				}
				None => {
					self.lookup_cache.truncate(0);
				}
			};
			Some(res.0)
		} else {
			None
		}
	}

	// Returns node and a reversed lookup path vector for the node, if found
	fn traverse_pending<'a: 'b, 'b>(
		&mut self,
		n: &'a mut Node,
	) -> Option<(&'b mut Node, Option<Vec<u64>>)> {
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
					Some(i) => {
						let ch = &mut cont.children[i];
						let id = ch.id;
						Some((
							ch,
							// Save target node ID, if it has been already
							// merged into the DOM and has one
							if id != 0 { Some(vec![id]) } else { None },
						))
					}
					None => {
						// If not found, go a level deeper on each node
						let parent_id = n.id; // Copy prevents borrow of n
						cont.children
							.iter_mut()
							.find_map(|ch| self.traverse_pending(ch))
							// If found node has a lookup path cache
							// building and this node has a known ID, append
							// it to the path cache vector
							.map(|res| {
								(
									res.0,
									match res.1 {
										Some(mut path) => {
											if parent_id != 0 {
												path.push(parent_id);
												Some(path)
											} else {
												None
											}
										}
										None => None,
									},
								)
							})
					}
				}
			}
		}
	}

	// TODO: Event handling
}
