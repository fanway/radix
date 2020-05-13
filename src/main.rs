mod trie;
mod radix;

fn main() {
    let mut radix_tree = radix::RadixTree::<String>::new(); 
    let vec = [("test", "test"), ("this", "this"), ("hashmap", "hashmap"), ("test1", "test1"), ("trie", "trie"), ("test12", "test12"), ("test123", "test123"), ("test21", "test21"), ("trie1", "trie1"), ("has", "has")];
    for i in vec.iter() {
        println!("{:?}", i);
        radix_tree.insert(i.0.to_string(), i.1.to_string());
    }
    radix_tree.print_edges();
    for i in vec.iter() {
        assert_eq!(i.0, radix_tree.find(i.0.to_string()).unwrap());
    }
    println!("{}", radix_tree.find("this".to_string()).unwrap());
    radix_tree.delete("trie".to_string());
    radix_tree.print_edges();
}
