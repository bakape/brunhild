use std::collections::BTreeMap;

static mut ID_COUNTER: u32 = 0;

// Generate a new unique view ID
fn new_ID() -> String {
	unsafe { ID_COUNTER += 1 };
	format!("brunhild-{}", unsafe { ID_COUNTER })
}

pub enum View<'a> {
	HTML(Box<HTMLView<'a>>),
	Parent(Box<ParentView<'a>>),
}

macro_rules! base_method {
	($view:expr, $method:ident) => (
		match $view {
			View::HTML(ref v) => v.$method(),
			View::Parent(ref v) => v.$method(),
		}
	)
}

pub type Attributes = BTreeMap<String, Option<String>>;

pub trait BaseView<'a> {
	fn tag(&self) -> &'a str;
	fn id(&self) -> Option<&'a str>;
	fn attrs(&self) -> Attributes;
	fn state(&self) -> u64;
}

pub trait HTMLView<'a>: BaseView<'a> {
	fn render(&self) -> String;
}

pub trait ParentView<'a>: BaseView<'a> {
	fn is_static(&self) -> bool;
	fn children(&self) -> &'a [View];
}

struct Node {
	tag: String,
	id: String,
	attrs: Attributes,
	state: u64,
	children: Vec<Node>,
}

impl Node {
	fn new(v: View) -> Node {
		Node {
			tag: String::from(base_method!(v, tag)),
			id: match base_method!(v, id) {
				Some(id) => String::from(id),
				None => new_ID(),
			},
			attrs: base_method!(v, attrs),
			state: base_method!(v, state),
			children: Vec::new(),
		}
	}
}
