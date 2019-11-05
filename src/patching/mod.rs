use std::fmt;

mod attrs;
mod node;
mod tokenizer;
mod util;

pub use node::{ElementOptions, Node, TextOptions};
pub use util::html_escape;

// Able to write itself as HTML to w
pub trait WriteHTMLTo {
	fn write_html_to<W: fmt::Write>(&mut self, w: &mut W) -> fmt::Result;
}
