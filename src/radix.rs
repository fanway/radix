use std::ops::{Index,IndexMut};
use std::collections::VecDeque;


struct Edge {
    target_node: usize,
    label: String
}

impl Edge {
    fn new(target_node: usize, label: String) -> Self {
        Self {
            target_node, 
            label
        }
    }
}

struct Node<T> {
    edges: Vec<usize>,
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

    fn insert(&mut self, val: T) -> usize {
        if self.arr_idx.is_empty() {
            self.arr.push(val);
            return self.arr.len() - 1;
        } else {
            self.arr[self.arr_idx[0]] = val;
            let idx = self.arr_idx[0];
            self.arr_idx.remove(0);
            return idx;
        }
    }

    fn is_empty(&self) -> bool {
        self.arr.is_empty()
    }

    fn delete(&mut self, idx: usize) {
        self.arr.remove(idx);
        self.arr_idx.push(idx);
    }
}

pub struct RadixTree<T> {
    nodes: Arena<Node<T>>,
    edges: Arena<Edge>
}

impl<T: std::default::Default + std::fmt::Debug + std::clone::Clone> RadixTree<T> {
    pub fn new() -> Self {
        let mut radix_tree = Self {
            nodes: Arena::<Node<T>>::new(),
            edges: Arena::<Edge>::new()
        };
        let mut zero_node = Node::new(T::default());
        zero_node.is_leaf = false;
        let zero_node_idx = radix_tree.nodes.insert(zero_node);
        radix_tree.edges.insert(Edge::new(zero_node_idx, "".to_string()));
        radix_tree
    }

    fn common_prefix(&self, first_str: &str, second_str: &str) -> Option<String> {
        let matching = first_str.chars().zip(second_str.chars()).take_while(|&(a, b)| a == b).count();
        if matching > 0 {
            return Some(first_str[..matching].to_string());
        }
        None
    }

    fn lookup(&self, key: String) -> (Ans, usize) {
        let mut idx = 0;
        let mut count = 0;
        let mut found = true;
        let mut node = &self.nodes[idx];
        while found && !node.is_leaf && count <= key.len() {
            found = false;
            for &e_idx in &node.edges {
                let edge = &self.edges[e_idx];
                // if the label have a prefix of a suffix of the key
                // example: looking for the word "testing" when we already
                // have "test", "tests", "testing"
                //      "test"
                //      /    \
                //   "s"     "ing"
                if &edge.label != "" && key[count..].starts_with(&edge.label) {
                    //println!("Key: {}, label: {}", &key[count..], &edge.label);
                    count += edge.label.len();
                    idx = e_idx;
                    found = true;
                    break;
                } else {
                    // in a case when a label might be longer we need to consider to split the node
                    // if there is a common prefix > 0
                    if let Some(cp) = self.common_prefix(&key[count..], &edge.label) {
                        count += cp.len() + key.len();
                        idx = e_idx;
                        break;
                    } else if &edge.label == "" && count == key.len() {
                        idx = e_idx;
                        found = true;
                        break;
                    }
                }
            }
            if found {
                node = &self.nodes[self.edges[idx].target_node];
            }
        }
        // if exact same key was found
        println!("{}, {}, {}, {}, {}, {}", idx, self.edges[idx].target_node, count, key.len(),
        self.nodes[self.edges[idx].target_node].is_leaf, found);
        if node.is_leaf && count == key.len() {
            return (Ans{exists:true, count}, idx);
        }
        (Ans{exists: false, count}, idx)
    }

    pub fn find(&self, key: String) -> Option<&T> {
        let (ans, idx) = self.lookup(key);
        if ans.exists {
            return Some(&self.nodes[self.edges[idx].target_node].value);
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    pub fn print_nodes(&self) {
        let mut q = VecDeque::new();
        q.push_front(0);
        while !q.is_empty() {
            let mut level_size = q.len();
            while level_size > 0 {
                let n = q.pop_front().unwrap();
                print!("{:#?}   ", self.nodes[n].value);
                for &edge in &self.nodes[n].edges {
                    q.push_back(self.edges[edge].target_node);
                }
                level_size -= 1;
            }
            println!();
        }
    }

    pub fn print_edges(&self) {
        let mut q = VecDeque::new();
        q.push_front(0);
        while !q.is_empty() {
            let mut level_size = q.len();
            while level_size > 0 {
                let n = q.pop_front().unwrap();
                print!("{:#?}   ", self.edges[n].label);
                for &edge in &self.nodes[self.edges[n].target_node].edges {
                    q.push_back(edge);
                }
                level_size -= 1;
            }
            println!();
        }
    }

    /*
    pub fn delete(&self, key: String) {
        let (ans, idx) = lookup(key.clone());
        if ans.exists {
            let target_node_idx = self.edges[idx].target_node;


    }
    */

    pub fn insert(&mut self, key: String, val: T) {
        let (ans, idx) = self.lookup(key.clone());
        let target_node_idx = self.edges[idx].target_node;
        if !ans.exists {
            if ans.count < key.len() {
                // case when we have to add new node with suffix
                let node_idx = self.nodes.insert(Node::new(val));
                let edge_idx = self.edges.insert(Edge::new(node_idx,key[ans.count..].to_string()));
                if self.nodes[target_node_idx].is_leaf {
                    let node_idx = self.nodes.insert(Node::new(self.nodes[target_node_idx].value.clone()));
                    self.nodes[target_node_idx].value = T::default();
                    let edge_idx = self.edges.insert(Edge::new(node_idx, "".to_string()));
                    self.nodes[target_node_idx].edges.push(edge_idx);
                }
                self.nodes[target_node_idx].edges.push(edge_idx);
                self.nodes[target_node_idx].is_leaf = false;
            } else {
                // case when we have to split node using common prefix
                //let split_node = self.nodes[target_node_idx].clone();
                let mut split_node = Node::new(T::default());
                split_node.is_leaf = false;
                let label = self.edges[idx].label.clone();
                let count = ans.count - key.len();
                
                self.edges[idx].label = key[..count].to_string();
                let edge_left = Edge::new(target_node_idx, label[count..].to_string());
                let edge_left_idx = self.edges.insert(edge_left);
                let new_node = Node::new(val);
                let new_node_idx = self.nodes.insert(new_node);
                let edge_right = Edge::new(new_node_idx, key[count..].to_string());
                let edge_right_idx = self.edges.insert(edge_right);
                split_node.edges.push(edge_left_idx);
                split_node.edges.push(edge_right_idx);
                let split_node_idx = self.nodes.insert(split_node);
                self.edges[idx].target_node = split_node_idx;
            }
        }
    }
}
