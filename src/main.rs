mod art;
mod radix;
mod trie;

fn main() {
    let mut art = art::Art::<u32>::new();
    println!("first insert ---------------------");
    art.insert(10, 10);
    println!("second insert ---------------------");
    art.insert(120, 20);
    println!("third insert ---------------------");
    art.insert(240, 30);
    println!("forth insert ---------------------");
    art.insert(480, 40);
    println!("fith insert ---------------------");
    art.insert(960, 50);
    println!("six insert ---------------------");
    art.insert(1920, 300);
    println!("seventh insert ---------------------");
    art.insert(3840, 301);
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
    println!("six find ---------------------");
    println!("{}", art.find(300).unwrap());
    println!("seventh find ---------------------");
    println!("{}", art.find(301).unwrap());
}
