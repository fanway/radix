mod trie;
mod radix;

use trie::TrieNode;

fn main() {
    let mut root: TrieNode<String> = TrieNode::new();
    for i in ["test", "this", "hashmap", "test1", "trie"].iter() {
        println!("{}", i);
        root.add(&mut i.chars().map(|s| {s.to_string()}));
    }
    println!("{}", root.find(&mut "hashap".chars().map(|s| {s.to_string()})));
}
