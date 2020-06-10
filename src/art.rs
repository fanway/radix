enum Node<T> {
    N4(Node4<T>),
    N16(Node16<T>),
    Leaf(LeafNode<T>),
}

const MAX_PREFIX_LEN: usize = 10;

struct Info {
    count: u8,
    partial: [u8; MAX_PREFIX_LEN]
}

struct Node4<T> {
    key: [u8; 4],
    child_pointers: [Option<Box<Node<T>>>; 4],
    info: Info
}

struct Node16<T> {
    key: [u8; 16],
    child_pointers: [Option<Box<Node<T>>>; 16],
    info: Info
}

struct LeafNode<T> {
    key: Vec<u8>,
    value: T
}

fn transform_u32(value: u32) -> [u8;4] {
    value.to_be_bytes()
}

impl<T> Node4<T> {
    fn new(prefix: &[u8]) -> Self {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(prefix);
        Self {
            key: [0; 4],
            child_pointers: [None; 4],
            info: Info {
                count: 0,
                partial
            }
        }
    }
    fn add(&mut self, node: Box<Node<T>>, key: u8) {
        let mut i = 0;
        while i < 3 {
            if key < self.key[i] {
                break;
            }
            i += 1;
        }
        self.key.swap(i+1, i);
        self.key[i] = key;
        self.child_pointers.swap(i+1, i);
        self.child_pointers[i] = Some(node);
    }
}

impl<T> LeafNode<T> {
    fn new(value: T, key: &[u8]) -> Self {
        Self {
            value,
            key: key.to_vec()
        }
    }
}

fn common_prefix(key: &[u8], partial: &[u8]) -> usize {
    key.iter().zip(partial.iter()).take_while(|&(a,b)| a == b).count()
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

    fn insert(&mut self, value: T, key: u32) {
        let key_bytes = key.to_be_bytes();
        if self.root.is_none() {
            self.root = Some(Box::new(Node::Leaf(LeafNode::new(value, &key_bytes))));
            return
        }
        let depth = 0;
        let iter_node = &self.root as *mut Option<Box<Node<T>>>;
        let parent_node = &self.root as *mut Option<Box<Node<T>>>;
        while Some(node) = iter_node {
            match node {
                Node::N4(node) => (),
                Node::Leaf(box ref node) => {
                    depth += common_prefix(&node.key, &key_bytes);
                    if depth == node.key.len() {
                        return
                    }
                    let new_leaf = Node::Leaf(LeafNode::new(value, &key_bytes[depth..]));
                    let mut new_node = Node4::new(&key_bytes[..depth]);
                    node.key = node.key[depth..].to_vec();
                    new_node.add(Box::new(new_leaf), key_bytes[depth]);
                    new_node.add(current_node, key_bytes[depth]);
                    parent_node = Some(Box::new(Node::N4(new_node)));
                }
            }
        }
    }
}
