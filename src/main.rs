#![feature(ptr_offset_from)]
mod art;
mod radix;
mod trie;

fn main() {
    let mut art = art::Art::<u32, u32>::new();
    println!("first insert ---------------------");
    art.insert(10, 10);
    println!("second insert ---------------------");
    art.insert(20, 120);
    println!("third insert ---------------------");
    art.insert(30, 240);
    println!("forth insert ---------------------");
    art.insert(40, 480);
    println!("fith insert ---------------------");
    art.insert(50, 960);
    println!("six insert ---------------------");
    art.insert(300, 1920);
    art.delete(300);
    println!("seventh insert ---------------------");
    art.insert(301, 3840);
    println!("first find ---------------------");
    println!("{}", art.find(10).unwrap());
    println!("second find ---------------------");
    println!("{}", art.find(20).unwrap());
    println!("third find ---------------------");
    println!("{}", art.find(30).unwrap());
    println!("fourth find ---------------------");
    println!("{}", art.find(40).unwrap());
    println!("fifth find ---------------------");
    println!("{}", art.find(50).unwrap());
    println!("seventh find ---------------------");
    println!("{}", art.find(301).unwrap());
}
