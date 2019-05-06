use super::attrs::Attrs;

/*
	Node used for constructing DOM trees for applying patches.

	This node type does not contain any binding to existing nodes in the DOM
	tree or in the pending patches tree. Such relation is determined during
	diffing.
*/
pub struct Node {
	tag: u64,
	class_set_id: u64,
	attrs: Attrs,
	pub children: Vec<Node>,
}
