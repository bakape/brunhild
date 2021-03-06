use super::util;
use std::cell::RefCell;
use std::fmt;

/*
Sorted list of predefined HTML tags and attributes to reduce allocations and
need for map checks.

Sourced from:
https://developer.mozilla.org/en-US/docs/Web/HTML/Element
https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes

NOTE: Some functions hard-code indexes into this. Do not change lightly.
constexpr when?
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
		for (i, ch) in s.chars().enumerate() {
			arr[i] = ch as u8;
		}
		Self {
			length: s.len() as u8,
			arr: arr,
		}
	}
}

impl AsRef<str> for ArrayString {
	fn as_ref(&self) -> &str {
		std::str::from_utf8(&self.arr[..self.length as usize]).unwrap()
	}
}

impl util::WriteHTMLTo for String {
	fn write_html_to<W: fmt::Write>(&mut self, w: &mut W) -> fmt::Result {
		w.write_str(&self)
	}
}

// Contains id->string and string->id mappings
#[derive(Default)]
struct Registry {
	id_gen: util::IDGenerator,
	small: util::TokenMap<ArrayString>,
	large: util::TokenMap<String>,
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
		match s.len() {
			0 => 0, // Don't store empty strings
			1..=15 => {
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

	/// Lookup string by token
	fn get_value(&self, k: u16) -> &str {
		if k == 0 {
			""
		} else if k <= PREDEFINED.len() as u16 {
			PREDEFINED[k as usize - 1]
		} else if util::IDGenerator::is_flagged(k) {
			self.large.get_value(k).as_ref()
		} else {
			self.small.get_value(k).as_ref()
		}
	}
}

// Convert string to token
#[inline]
pub fn tokenize(s: &str) -> u16 {
	if let Ok(i) = PREDEFINED.binary_search(&s) {
		return i as u16 + 1;
	}
	util::with_global_mut(&REGISTRY, |r| r.tokenize(s))
}

// Lookup value by token and pass it to f
pub fn get_value<F, R>(k: u16, f: F) -> R
where
	F: FnOnce(&str) -> R,
{
	util::with_global(&REGISTRY, |r| f(r.get_value(k)))
}
