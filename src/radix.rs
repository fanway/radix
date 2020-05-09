use std::rc::Rc;
use std::ops::{Index,IndexMut};

struct Edge<T> {
    target_node: Rc<Box<Node<T>>>,
    label: String
}

impl<T> Edge<T> {
    fn new(target_node: Node<T>, label: String) -> Self {
        Self {
            target_node: Rc::new(Box::new(target_node)),
            label
        }
    }
}

struct Node<T> {
    edges: Vec<Rc<Box<Edge<T>>>>,
    value: T,
    is_leaf: bool
}

impl<T> Node<T> {
    fn new(value: T) -> Self {
        Self {
            edges: vec![],
            value,
            is_leaf: true
        }
    }
}

struct Ans {
    exists: bool,
    count: usize 
}

struct Arena<T> {
    arr: Vec<T>,
    arr_idx: Vec<usize>,
}

impl<T> Index<usize> for Arena<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.arr[index]
    }
}

impl<T> IndexMut<usize> for Arena<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.arr[index]
    }
}

impl<T> Arena<T> {
    fn new() -> Self {
        Self {
            arr: vec![],
            arr_idx: vec![]
        }
    }

    fn new_with_size(size: usize) -> Self {
        let mut arr_idx = Vec::with_capacity(size);
        for i in 0..size {
            arr_idx[i] = i;
        }
        Self {
            arr: Vec::with_capacity(size),
            arr_idx
        }
    }

    fn insert(&mut self, val: T) {
        if self.arr_idx.is_empty() {
            self.arr.push(val);
        } else {
            self.arr[self.arr_idx[0]] = val;
            self.arr_idx.remove(0);
        }
    }

    fn delete(&mut self, idx: usize) {
        self.arr.remove(idx);
        self.arr_idx.push(idx);
    }
}

fn lookup<T>(mut search_node: Rc<Box<Node<T>>>, key: String) -> (Ans, Rc<Box<Node<T>>>) {
    let mut count = 0;
    while !search_node.is_leaf && count < key.len() {
        for e in &search_node.edges {
            if e.label.starts_with(&key[count..]) {
                count += e.label.len();
                search_node = Rc::clone(&e.target_node);
                break
            }
        }
    }
    if search_node.is_leaf && count == key.len() {
        return (Ans{exists:true, count}, search_node);
    }
    (Ans{exists: false, count}, search_node)
}

fn find<T: Clone>(search_node: Rc<Box<Node<T>>>, key: String) -> Option<T> {
    if let (ans, node) = lookup(search_node, key) {
        if ans.exists {
            return Some(node.value.clone());
        }
    }
    None
}

fn insert<T>(root: Rc<Box<Node<T>>>, key: String, val: T) {
    if let (ans, node) = lookup(root, key) {
        if !ans.exists {
            if ans.count < key.len() {
                let new_node = Node::new(val);
                let new_edge = Edge::new(new_node, key[ans.count..].to_string());
                node.edges.push(Rc::new(Box::new(new_edge)));
            }
        }
    }
}
