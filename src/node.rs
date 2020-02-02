use super::attrs::Attrs;
use super::tokenizer;
use super::util;
use super::util::WriteHTMLTo;
use std::collections::HashMap;
use std::fmt;
use wasm_bindgen::JsValue;

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

// Internal contents of an Element
struct ElementContents {
	// Token for the node's tag
	tag: u16,

	// Node attributes, excluding "id" and "class".
	// "id" is used internally for node addressing and can not be set.
	// to set "class" used the dedicated methods.
	attrs: Attrs,

	// Children of Node
	children: Vec<Node>,
}

impl Default for ElementContents {
	fn default() -> Self {
		Self {
			tag: tokenizer::tokenize("div"),
			attrs: Default::default(),
			children: Default::default(),
		}
	}
}

// Node used for constructing DOM trees for applying patches and representing
// the browser DOM state.
#[derive(Default)]
pub struct Node {
	// ID of DOM element the node is representing. Can be 0 in nodes not yet
	// patched into the DOM.
	id: u64,

	// Key used to identify the same node, during potentially destructive
	// patching. Only set, if this node requires persistance, like maintaining
	// user input focus or selections.
	key: Option<u64>,

	contents: NodeContents,

	// Lazy getter for corresponding JS Element object
	element: util::LazyElement,
}

// Options for constructing an Element Node. This struct has separate lifetimes
// for each field, so that some of these can have static lifetimes and thus not
// require runtime allocation.
pub struct ElementOptions<'t, 'a> {
	// Element HTML tag
	pub tag: &'t str,

	// Kee used to identify the same node, during potentially destructive
	// patching. Only set, if this node requires persistance, like maintaining
	// user input focus or selections.
	pub key: Option<u64>,

	// List of element attributes
	pub attrs: &'a [&'a (&'a str, &'a str)],
}

impl<'t, 'a> Default for ElementOptions<'t, 'a> {
	fn default() -> Self {
		Self {
			tag: "div",
			key: None,
			attrs: &[],
		}
	}
}

impl<'t, 'a> ElementOptions<'t, 'a> {
	// Shorthand for constructing new element with nothing but a tag
	#[inline]
	pub fn new(tag: &'t str) -> ElementOptions<'t, 'a> {
		ElementOptions {
			tag: tag,
			..Default::default()
		}
	}

	// Shorthand for constructing new element with attributes
	#[inline]
	pub fn with_attrs(
		tag: &'t str,
		attrs: &'a [&'a (&'a str, &'a str)],
	) -> ElementOptions<'t, 'a> {
		ElementOptions {
			tag: tag,
			attrs: attrs,
			..Default::default()
		}
	}
}

// Options for constructing a text Node
pub struct TextOptions<'a> {
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
			escape: true,
			text: "",
			key: None,
		}
	}
}

impl Node {
	// Create an Element Node.
	//
	// # Panics
	//
	// Setting element "id" attributes is not supported. Panics, if key == "id".
	#[inline]
	pub fn element(opts: &ElementOptions) -> Self {
		Self {
			contents: NodeContents::Element(ElementContents {
				tag: tokenizer::tokenize(opts.tag),
				attrs: super::attrs::Attrs::new(opts.attrs),
				..Default::default()
			}),
			key: opts.key,
			..Default::default()
		}
	}

	// Create an Element Node with children
	#[inline]
	pub fn with_children(opts: &ElementOptions, children: Vec<Node>) -> Self {
		Self {
			contents: NodeContents::Element(ElementContents {
				tag: tokenizer::tokenize(opts.tag),
				attrs: super::attrs::Attrs::new(opts.attrs),
				children: children,
			}),
			key: opts.key,
			..Default::default()
		}
	}

	// Create a text Node with set inner content
	#[inline]
	pub fn text(opts: &TextOptions) -> Self {
		Self {
			contents: NodeContents::Text(if opts.escape {
				util::html_escape(opts.text.into())
			} else {
				opts.text.into()
			}),
			key: opts.key,
			..Default::default()
		}
	}

	// Mount Node as passed Element.
	// Sets the element's ID attribute and takes ownership of Element.
	pub fn mount_as(&mut self, el: web_sys::Element) -> Result<(), JsValue> {
		el.set_outer_html(&self.html()?);
		self.element.element = Some(el);
		Ok(())
	}

	// Mount Node after as last child of parent
	pub fn append_to(
		&mut self,
		parent: web_sys::Element,
	) -> Result<(), JsValue> {
		self.mount(parent, "beforeend")
	}

	// Mount Node after as first child of parent
	pub fn prepend_to(
		&mut self,
		parent: web_sys::Element,
	) -> Result<(), JsValue> {
		self.mount(parent, "afterbegin")
	}

	// Mount Node after as previous sibling of parent
	pub fn mount_before(
		&mut self,
		parent: web_sys::Element,
	) -> Result<(), JsValue> {
		self.mount(parent, "beforebegin")
	}

	// Mount Node after as next sibling of parent
	pub fn mount_after(
		&mut self,
		parent: web_sys::Element,
	) -> Result<(), JsValue> {
		self.mount(parent, "afterend")
	}

	fn mount(
		&mut self,
		parent: web_sys::Element,
		mode: &str,
	) -> Result<(), JsValue> {
		parent.insert_adjacent_html(mode, &self.html()?)
	}

	// Return the DOM element ID of node
	pub fn element_id(&self) -> String {
		format!("bh-{}", self.id)
	}

	// Patch possibly changed subtree into self and apply changes to the DOM.
	// Node must be already mounted.
	pub fn patch(&mut self, new: Node) -> Result<(), JsValue> {
		if self.id == 0 {
			return Err("node not mounted yet".into());
		}

		// Check, if nodes are considered similar enough to be merged and not
		// replaced destructively
		if self.key != new.key
			|| match &self.contents {
				NodeContents::Text(_) => match &new.contents {
					NodeContents::Element(_) => true,
					NodeContents::Text(_) => false,
				},
				NodeContents::Element(cont) => match &new.contents {
					NodeContents::Text(_) => true,
					NodeContents::Element(new_cont) => new_cont.tag != cont.tag,
				},
			} {
			return Node::replace_node(self, new);
		}

		self.key = new.key;
		match &mut self.contents {
			NodeContents::Text(ref mut old_text) => {
				if let NodeContents::Text(new_text) = &new.contents {
					if old_text != new_text {
						*old_text = new_text.clone();
						self.element.get()?.set_text_content(Some(old_text));
					}
				}
			}
			NodeContents::Element(ref mut old_cont) => {
				if let NodeContents::Element(new_cont) = new.contents {
					old_cont.attrs.patch(&mut self.element, new_cont.attrs)?;

					Node::patch_children(
						&mut self.element,
						&mut old_cont.children,
						new_cont.children,
					)?;
				}
			}
		};
		Ok(())
	}

	// Completely replace old node and its subtree with new one
	fn replace_node(&mut self, new: Node) -> Result<(), JsValue> {
		self.key = new.key;
		self.contents = new.contents;
		self.element.get()?.set_outer_html(&self.html()?);
		Ok(())
	}

	// Diff and patch 2 child lists
	fn patch_children(
		parent: &mut util::LazyElement,
		old: &mut Vec<Node>,
		new: Vec<Node>,
	) -> Result<(), JsValue> {
		let mut old_it = old.iter_mut().peekable();
		let mut new_it = new.into_iter().peekable();
		let mut i = 0;

		// First patch all matching children. Most of the time child lists will
		// match, so this is the hottest loop.
		loop {
			let old_ch = old_it.peek();
			let new_ch = new_it.peek();
			if let Some(old_ch) = old_ch {
				if let Some(new_ch) = new_ch {
					if (old_ch.key.is_some() || new_ch.key.is_some())
						&& old_ch.key != new_ch.key
					{
						return Node::patch_children_by_key(
							parent, old, i, new_it,
						);
					}

					old_it.next().unwrap().patch(new_it.next().unwrap())?;
					i += 1;
					continue;
				}
			}
			break;
		}

		// Handle mismatched node counts using appends or deletes
		if new_it.peek().is_some() {
			// Append new nodes to end

			let mut w = util::Appender::new();
			old.reserve(new_it.size_hint().0);
			for mut new_ch in new_it {
				new_ch.write_html_to(&mut w).map_err(util::cast_error)?;
				old.push(new_ch);
			}
			parent.get()?.insert_adjacent_html("beforeend", &w.dump())?;
		} else if old_it.peek().is_some() {
			// Remove nodes from end

			for old_ch in old_it {
				old_ch.element.get()?.remove();
			}
			old.truncate(i);
		}

		Ok(())
	}

	// Match and patch nodes by key, if any
	fn patch_children_by_key(
		parent: &mut util::LazyElement,
		old: &mut Vec<Node>,
		mut i: usize,
		new_it: std::iter::Peekable<std::vec::IntoIter<Node>>,
	) -> Result<(), JsValue> {
		// Map old children by key
		let mut old_by_key = HashMap::<u64, Node>::new();
		let mut to_remove = Vec::<Node>::new();
		for ch in old.split_off(i) {
			match ch.key {
				Some(k) => {
					old_by_key.insert(k, ch);
				}
				None => {
					to_remove.push(ch);
				}
			}
		}

		// Insert new HTML into the DOM efficiently in buffered chunks
		old.reserve(new_it.size_hint().0);
		let mut w = util::Appender::new();
		let mut buffered = 0;

		let flush = |w: &mut util::Appender,
		             i: &mut usize,
		             buffered: &mut usize,
		             old: &mut Vec<Node>,
		             parent: &mut util::LazyElement|
		 -> Result<(), JsValue> {
			if *buffered == 0 {
				return Ok(());
			}

			let html = w.dump();
			w.clear();
			if *i == 0 {
				parent.get()?.insert_adjacent_html("afterbegin", &html)?;
			} else {
				old[*i]
					.element
					.get()?
					.insert_adjacent_html("afterend", &html)?;
			}
			*i += *buffered;
			*buffered = 0;

			Ok(())
		};

		for mut new_ch in new_it {
			if let Some(k) = new_ch.key {
				if let Some(mut old_ch) = old_by_key.remove(&k) {
					flush(&mut w, &mut i, &mut buffered, old, parent)?;

					let el = old_ch.element.get()?;
					if i == 0 {
						parent
							.get()?
							.insert_adjacent_element("afterbegin", &el)?;
					} else {
						old[i]
							.element
							.get()?
							.insert_adjacent_element("afterend", &el)?;
					}
					old_ch.patch(new_ch)?;
					old.push(old_ch);
					i += 1;
					continue;
				}
			}
			new_ch.write_html_to(&mut w).map_err(util::cast_error)?;
			old.push(new_ch);
			buffered += 1;
		}
		flush(&mut w, &mut i, &mut buffered, old, parent)?;

		// Remove any unmatched old children
		for mut ch in to_remove
			.into_iter()
			.chain(old_by_key.into_iter().map(|(_, v)| v))
		{
			ch.element.get()?.remove();
		}

		Ok(())
	}

	// Set new element ID on self
	fn new_id(&mut self) {
		static mut COUNTER: u64 = 0;
		unsafe { COUNTER += 1 };
		self.id = unsafe { COUNTER };
	}

	// Ensure Node has an element ID set
	fn ensure_id(&mut self) {
		if self.id == 0 {
			self.new_id();
			self.element.id = self.id;
		}
	}

	// Format element and subtree as HTML
	pub fn html(&mut self) -> Result<String, JsValue> {
		let mut w = util::Appender::new();
		if let Err(e) = self.write_html_to(&mut w) {
			return Err(util::cast_error(e));
		}
		Ok(w.dump())
	}
}

impl util::WriteHTMLTo for Node {
	fn write_html_to<W: fmt::Write>(&mut self, w: &mut W) -> fmt::Result {
		self.ensure_id();

		match &mut self.contents {
			NodeContents::Text(ref text) => {
				write!(w, "<span id=\"bh-{}\">{}</span>", self.id, text)
			}
			NodeContents::Element(ref mut cont) => {
				let id = self.id;
				tokenizer::get_value(cont.tag, |tag| {
					write!(w, "<{} id=\"bh-{}\"", tag, id)
				})?;
				cont.attrs.write_html_to(w)?;
				w.write_char('>')?;

				match cont.tag {
					// <br>, <hr> and <wbr> must not be closed.
					// Some browsers will interpret that as 2 tags.
					36 | 124 | 282 => {
						return Ok(());
					}
					_ => {
						for ch in cont.children.iter_mut() {
							ch.write_html_to(w)?;
						}
					}
				};

				tokenizer::get_value(cont.tag, |tag| write!(w, "</{}>", tag))
			}
		}
	}
}

#[test]
fn create_element_node() {
	#[allow(unused)]
	let node = Node::element(&ElementOptions {
		tag: "span",
		attrs: &[
			&(
				// Test long string allocators
				"loooooooooooooooooooooooooooooooooooooooooooooooooooooong",
				"caaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaat",
			),
			&("classes", "class1 class2"),
			&("disabled", ""),
			&("width", "64"),
		],
		..Default::default()
	});
}

#[test]
fn create_element_node_with_children() {
	#[allow(unused)]
	let node = Node::with_children(
		&ElementOptions::with_attrs(
			"span",
			&[&("disabled", ""), &("width", "64")],
		),
		vec![Node::element(&ElementOptions {
			tag: "span",
			attrs: &[&("disabled", ""), &("width", "64")],
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
