use std::collections::HashMap;
use std::default::Default;
use std::cmp::Eq;
use std::hash::Hash;

pub struct TrieNode<T> {
    next: HashMap<T, TrieNode<T>>,
    end: bool
}

impl<T: Default + Eq + Hash + Clone> TrieNode<T> {
    pub fn new() -> Self {
        Self {
            next: HashMap::new(),
            end: false
        }
    }

    pub fn add(&mut self, s: &mut dyn Iterator<Item = T>) {
        let mut n = self;
        for c in s {
            if n.end {
                break;
            }
            n = n.next.entry(c).or_insert(TrieNode::new());
        }
        n.end = true;
    }
    pub fn find(&self, s: &mut dyn Iterator<Item = T>) -> bool {
        let mut n = self;
        for c in s {
            match n.next.get(&c) {
                Some(node) => n = node,
                None => return false
            }
        }
        n.end
    }
}
