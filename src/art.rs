enum Node<T> {
    N4(Box<Node4>),
    N16(Box<Node16>),
    Leaf(Box<LeafNode<T>>),
}

struct Node4 {
    key: [u8; 4],
    count: u8,
    child_pointers: [Option<Box<Node4>>; 4]
}

struct Node16 {
    key: [u8; 16],
    count: u8,
    child_pointers: [Option<Box<Node4>>; 16]
}

struct LeafNode<T> {
    value: T
}

impl<T> LeafNode<T> {
    fn new(&self, value) -> Self {
        Self {
            value
        }
    }
}

struct Art<T> {
    root: Option<Box<Node<T>>>
}

impl<T> Art<T> {
    fn new(&self) -> Self {
        Self {
            root: None
        }
    }

    fn insert(&self, value: T) {
        if root.is_none() {
            root = Some(Box::new(LeafNode::new(value)));
        }

        if let Some(node) = root {
            
        }
    }
}