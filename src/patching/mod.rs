// TODO: Remove this
#![allow(unused)]

use std::fmt;
use web_sys;

pub mod attrs;
mod classes;
pub mod node;
mod tokenizer;
mod util;
pub use util::html_escape;

// Able to write itself as HTML to w
pub trait WriteHTMLTo {
	fn write_html_to<W: fmt::Write>(&self, w: &mut W) -> fmt::Result;
}
