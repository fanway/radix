mod trie;
mod radix;

use trie::TrieNode;
use radix::*;
use std::error::Error;

fn main() {
    let mut radix_tree = radix::RadixTree::<String>::new(); 
    let vec = [("test", "test"), ("this", "this"), ("hashmap", "hashmap"), ("test1", "test1"), ("trie", "trie"), ("test12", "test12")];
    for i in vec.iter() {
        println!("{:?}", i);
        radix_tree.insert(i.0.to_string(), i.1.to_string());
    }
    for i in vec.iter() {
        assert_eq!(i.0, radix_tree.find(i.0.to_string()).unwrap());
    }
    radix_tree.print_edges();
    println!("{}", radix_tree.find("this".to_string()).unwrap());
}
