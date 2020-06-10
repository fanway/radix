mod trie;
mod radix;
mod art;

fn main() {
    let mut art = art::Art::<u32>::new();
    art.insert(10, 10);
    art.insert(120, 20);
    println!("{}", art.find(10).unwrap());
    println!("{}", art.find(20).unwrap());
}
