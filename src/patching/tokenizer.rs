use super::util;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::AsRef;
use std::fmt;
use std::hash::Hash;

/*
Sorted list of predefined HTML tags and attributes to reduce allocations and
need for map checks.

Sourced from:
https://developer.mozilla.org/en-US/docs/Web/HTML/Element
https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes

NOTE: Some functions harcode indexes into this. Do not change lightly.
*/
static PREDEFINED: [&'static str; 285] = [
	"a",
	"abbr",
	"accept",
	"accept-charset",
	"accesskey",
	"acronym",
	"action",
	"address",
	"align",
	"allow",
	"alt",
	"applet",
	"applet",
	"area",
	"article",
	"aside",
	"async",
	"audio",
	"autocapitalize",
	"autocomplete",
	"autofocus",
	"autoplay",
	"b",
	"background",
	"base",
	"basefont",
	"bdi",
	"bdo",
	"bgcolor",
	"bgsound",
	"big",
	"blink",
	"blockquote",
	"body",
	"border",
	"br",
	"buffered",
	"button",
	"canvas",
	"caption",
	"center",
	"challenge",
	"charset",
	"checked",
	"cite",
	"cite",
	"class",
	"code",
	"code",
	"codebase",
	"col",
	"colgroup",
	"color",
	"cols",
	"colspan",
	"command",
	"content",
	"content",
	"content",
	"contenteditable",
	"contextmenu",
	"controls",
	"coords",
	"crossorigin",
	"csp",
	"data",
	"data",
	"data-*",
	"datalist",
	"datetime",
	"dd",
	"decoding",
	"default",
	"defer",
	"del",
	"details",
	"dfn",
	"dialog",
	"dir",
	"dir",
	"dir",
	"dirname",
	"disabled",
	"div",
	"dl",
	"download",
	"draggable",
	"dropzone",
	"dt",
	"element",
	"element",
	"em",
	"embed",
	"enctype",
	"enterkeyhint",
	"fieldset",
	"figcaption",
	"figure",
	"font",
	"footer",
	"for",
	"form",
	"form",
	"formaction",
	"formenctype",
	"formmethod",
	"formnovalidate",
	"formtarget",
	"frame",
	"frameset",
	"h1",
	"h2",
	"h3",
	"h4",
	"h5",
	"h6",
	"head",
	"header",
	"headers",
	"height",
	"hgroup",
	"hidden",
	"high",
	"hr",
	"href",
	"hreflang",
	"html",
	"http-equiv",
	"i",
	"icon",
	"id",
	"iframe",
	"image",
	"img",
	"importance",
	"input",
	"inputmode",
	"ins",
	"integrity",
	"intrinsicsize",
	"isindex",
	"ismap",
	"itemprop",
	"kbd",
	"keygen",
	"keytype",
	"kind",
	"label",
	"label",
	"lang",
	"language",
	"legend",
	"li",
	"link",
	"list",
	"listing",
	"loading",
	"loop",
	"low",
	"main",
	"main",
	"manifest",
	"map",
	"mark",
	"marquee",
	"max",
	"maxlength",
	"media",
	"menu",
	"menuitem",
	"menuitem",
	"meta",
	"meter",
	"method",
	"min",
	"minlength",
	"multicol",
	"multiple",
	"muted",
	"name",
	"nav",
	"nextid",
	"nobr",
	"noembed",
	"noembed",
	"noframes",
	"noscript",
	"novalidate",
	"object",
	"ol",
	"open",
	"optgroup",
	"optimum",
	"option",
	"output",
	"p",
	"param",
	"pattern",
	"picture",
	"ping",
	"placeholder",
	"plaintext",
	"poster",
	"pre",
	"preload",
	"progress",
	"q",
	"radiogroup",
	"rb",
	"readonly",
	"referrerpolicy",
	"rel",
	"required",
	"reversed",
	"rows",
	"rowspan",
	"rp",
	"rt",
	"rtc",
	"ruby",
	"s",
	"samp",
	"sandbox",
	"scope",
	"scoped",
	"script",
	"section",
	"select",
	"selected",
	"shadow",
	"shadow",
	"shape",
	"size",
	"sizes",
	"slot",
	"slot",
	"small",
	"source",
	"spacer",
	"span",
	"span",
	"spellcheck",
	"src",
	"srcdoc",
	"srclang",
	"srcset",
	"start",
	"step",
	"strike",
	"strong",
	"style",
	"style",
	"sub",
	"summary",
	"summary",
	"sup",
	"tabindex",
	"table",
	"target",
	"tbody",
	"td",
	"template",
	"textarea",
	"tfoot",
	"th",
	"thead",
	"time",
	"title",
	"title",
	"tr",
	"track",
	"translate",
	"tt",
	"tt",
	"type",
	"u",
	"ul",
	"usemap",
	"value",
	"var",
	"video",
	"wbr",
	"width",
	"wrap",
	"xmp",
];

thread_local! {
	static REGISTRY: RefCell<Registry> = RefCell::new(Registry::new());
}

// Storage for small (len <= 15) strings without allocating extra heap memory
#[derive(Default, PartialEq, Eq, Hash, Clone)]
struct ArrayString {
	length: u8,
	arr: [u8; 15],
}

impl ArrayString {
	fn new(s: &str) -> Self {
		let mut arr: [u8; 15] = Default::default();
		arr.copy_from_slice(s.as_bytes());
		Self {
			length: s.len() as u8,
			arr: arr,
		}
	}
}

impl super::WriteHTMLTo for ArrayString {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		w.write_str(std::str::from_utf8(&self.arr).unwrap())
	}
}

impl super::WriteHTMLTo for String {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
		w.write_str(&self)
	}
}

// Contains id->string and string->id mappings
#[derive(Default)]
struct Registry {
	id_gen: util::IDGenerator,
	small: util::TokenMap<ArrayString>,
	large: util::PointerTokenMap<String>,
}

impl Registry {
	fn new() -> Self {
		Self {
			id_gen: util::IDGenerator::new(PREDEFINED.len() as u16),
			..Default::default()
		}
	}

	// Convert string to token
	fn tokenize(&mut self, s: &str) -> u16 {
		if let Ok(i) = PREDEFINED.binary_search(&s) {
			return i as u16 + 1;
		}
		match s.len() {
			0 => 0, // Don't store empty strings
			1...15 => {
				let v = ArrayString::new(s);
				match self.small.get_token(&v) {
					Some(t) => *t,
					None => {
						let t = self.id_gen.new_id(false);
						self.small.insert(t, v);
						t
					}
				}
			}
			_ => {
				let v = String::from(s);
				match self.large.get_token(&v) {
					Some(t) => *t,
					None => {
						let t = self.id_gen.new_id(true);
						self.large.insert(t, v);
						t
					}
				}
			}
		}
	}

	// Lookup string by token and write it to w
	fn write_html_to<W: fmt::Write>(&self, k: u16, w: &mut W) -> fmt::Result {
		if k == 0 {
			Ok(())
		} else if k <= PREDEFINED.len() as u16 {
			w.write_str(PREDEFINED[k as usize - 1])
		} else {
			if util::IDGenerator::is_flagged(k) {
				self.large.write_html_to(k, w)
			} else {
				self.small.write_html_to(k, w)
			}
		}
	}
}

// Convert string to token
pub fn tokenize(s: &str) -> u16 {
	util::with_global(&REGISTRY, |r| r.tokenize(s))
}

// Lookup token and write value to w
pub fn write_html_to<W: fmt::Write>(k: u16, w: &mut W) -> fmt::Result {
	util::with_global(&REGISTRY, |r| r.write_html_to(k, w))
}
