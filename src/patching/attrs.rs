use std::collections::BTreeMap;

// Compressed attribute storage with manipulation functions
pub struct Attrs {
	map: BTreeMap<u64, Value>,
}

// Contains a value stored in one of three storage methods for attribute values
enum Value {
	// Tokenized string value
	StringToken(u64),

	// Tokenized set of classes
	ClassSet(u64),

	// Untokenized string. Used to store values too dynamic to benefit from
	// tokenization in most use cases.
	Untokenized(String),
}

impl Attrs {
	fn new() -> Self {
		Attrs {
			map: BTreeMap::new(),
		}
	}

	// Sets an attribute value of a Node.
	// Setting element id attributes is not supported and results in a panic.
	//
	// # Panics
	//
	// Panics if key="id"
	fn set(key: &str, val: &str) {
		if key == "id" {
			panic!("setting element id is not supported");
		}

		// TODO: If passed a "class", forward to class setting method

		unimplemented!()
	}
}
