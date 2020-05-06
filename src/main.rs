use std::collections::HashMap;
use std::default::Default;
use std::cmp::Eq;
use std::hash::Hash;


struct NaiveRadixTreeNode<T> {
    next: HashMap<T, NaiveRadixTreeNode<T>>,
    end: bool
}

impl<T: Default + Eq + Hash + Clone> NaiveRadixTreeNode<T> {
    fn new() -> Self {
        Self {
            next: HashMap::new(),
            end: false
        }
    }

    fn add(&mut self, s: &mut dyn Iterator<Item = T>) {
        let mut n = self;
        for c in s {
            if n.end {
                break;
            }
            n = n.next.entry(c).or_insert(NaiveRadixTreeNode::new());
        }
        n.end = true;
    }
    fn find(&self, s: &mut dyn Iterator<Item = T>) -> bool {
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

fn main() {
    let mut root: NaiveRadixTreeNode<String> = NaiveRadixTreeNode::new();
    for i in ["test", "this", "hashmap", "test1", "trie"].iter() {
        println!("{}", i);
        root.add(&mut i.chars().map(|s| {s.to_string()}));
    }
    println!("{}", root.find(&mut "hashap".chars().map(|s| {s.to_string()})));
}
