mod trie;
mod radix;
mod art;

fn main() {
    let mut art = art::Art::<u32>::new();
    println!("first insert ---------------------");
    art.insert(10, 10);
    println!("second insert ---------------------");
    art.insert(120, 20);
    println!("third insert ---------------------");
    art.insert(240, 30);
    println!("first find ---------------------");
    println!("{}", art.find(10).unwrap());
    println!("second find ---------------------");
    println!("{}", art.find(20).unwrap());
    println!("third find ---------------------");
    println!("{}", art.find(30).unwrap());
}
