use std::fmt;

mod attrs;
mod classes;
mod node;
mod patching;
mod tokenizer;
mod util;

pub use node::{ElementOptions, Handle, Node, TextOptions};
pub use patching::set_root;
pub use util::html_escape;

// Able to write itself as HTML to w
pub trait WriteHTMLTo {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result;
}
