mod patching;
mod utils;

pub use patching::attrs::Attrs;
pub use patching::node::Node;
pub use patching::html_escape;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
