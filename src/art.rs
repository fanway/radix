use rand::Rng;
use std::ptr;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[derive(Debug)]
enum Node<T> {
    N4(Node4<T>),
    N16(Node16<T>),
    Leaf(LeafNode<T>),
}

impl<T> Node<T> {
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        match self {
            Node::N4(n) => {
                n.add(node, key, depth);
            }
            _ => (),
        }
    }
}

const MAX_PREFIX_LEN: usize = 10;

#[derive(Debug)]
struct Info {
    count: usize,
    partial: [u8; MAX_PREFIX_LEN],
    partial_len: usize,
}

#[derive(Debug)]
struct Node4<T> {
    key: [u8; 4],
    child_pointers: [*mut Node<T>; 4],
    info: Info,
}

#[derive(Debug)]
struct Node16<T> {
    key: [u8; 16],
    child_pointers: [*mut Node<T>; 16],
    info: Info,
}

#[derive(Debug)]
struct LeafNode<T> {
    key: Vec<u8>,
    value: T,
}

fn transform(value: u32) -> [u8; 4] {
    value.to_be_bytes()
}

impl<T> Node4<T> {
    fn new(prefix: &[u8]) -> Self {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(prefix);
        Self {
            key: [0; 4],
            child_pointers: [std::ptr::null_mut(); 4],
            info: Info {
                count: 0,
                partial,
                partial_len: min,
            },
        }
    }
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        let mut i: usize = 0;
        while i < 3 && i < self.info.count {
            if key[depth] < self.key[i] {
                break;
            }
            i += 1;
        }
        if i != 3 && self.info.count != 0 {
            self.key.swap(i + 1, i);
            self.child_pointers.swap(i + 1, i);
        }
        self.info.count += 1;
        let min = std::cmp::min(key[..depth].len(), MAX_PREFIX_LEN);
        for j in 0..min {
            self.info.partial[j] = key[j];
            self.info.partial_len = min;
        }
        self.key[i] = key[depth];
        self.child_pointers[i] = node;
    }
}

impl<T> Node16<T> {
    fn new(prefix: &[u8]) -> Self {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(prefix);
        Self {
            key: [0; 16],
            child_pointers: [std::ptr::null_mut(); 16],
            info: Info {
                count: 0,
                partial,
                partial_len: min,
            },
        }
    }

    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        let mask = (1 << self.info.count) - 1;
        unsafe {
            // Compare the key to all 16 stored keys
            let cmp = _mm_cmplt_epi8(
                _mm_set1_epi8(key[depth] as i8),
                _mm_load_si128((&self.key).as_ptr() as *const __m128i),
            );

            // Use a mask to ignore children that don't exist
            let bitfield = _mm_movemask_epi8(cmp) & mask;
            let mut i: usize = 0;
            if bitfield != 0 {
                i = bitfield.trailing_zeros() as usize;
                self.key.swap(i + 1, i);
                self.child_pointers.swap(i + 1, i);
            } else {
                i = self.info.count;
            }
            self.key[i] = key[depth];
            self.child_pointers[i] = node;
            self.info.count += 1;
        }
    }
}

impl<T> LeafNode<T> {
    fn new(value: T, key: &[u8]) -> Self {
        Self {
            value,
            key: key.to_vec(),
        }
    }
}

fn common_prefix(key: &[u8], partial: &[u8]) -> usize {
    key.iter()
        .zip(partial.iter())
        .take_while(|&(a, b)| a == b)
        .count()
}

pub struct Art<T> {
    root: *mut Node<T>,
}

impl<T: Clone + std::fmt::Debug> Art<T> {
    pub fn new() -> Self {
        Self {
            root: std::ptr::null_mut(),
        }
    }

    fn find_child(&self, node: &Node<T>, key: u8) -> Option<*mut Node<T>> {
        match node {
            Node::N4(node) => {
                for i in 0..node.info.count as usize {
                    if key == node.key[i] {
                        return Some(node.child_pointers[i]);
                    }
                }
            }
            Node::N16(_) => (),
            Node::Leaf(_) => (),
        }
        None
    }

    pub fn find(&self, key: u32) -> Option<T> {
        let mut iter_node = self.root;
        println!("iter_node: {:?}", unsafe { &*iter_node });
        let key_bytes = key.to_be_bytes();
        let mut depth = 0;
        while !iter_node.is_null() {
            match unsafe { &*iter_node } {
                Node::N4(node) => {
                    depth += common_prefix(&node.info.partial[..node.info.partial_len], &key_bytes);
                    //println!("{:?}, {:?}", &node.info.partial[..node.info.partial_len], key_bytes);
                    println!("{:?}", node);
                    println!("{:?}", unsafe { &*node.child_pointers[0] });
                    println!("{:?}", unsafe { &*node.child_pointers[1] });
                    if let Some(n) = self.find_child(unsafe { &*iter_node }, key_bytes[depth]) {
                        iter_node = n;
                    } else {
                        break;
                    }
                }
                Node::N16(node) => (),
                Node::Leaf(node) => {
                    let cm = common_prefix(&node.key, &key_bytes);
                    println!("test: {}", cm);
                    if cm == node.key.len() {
                        return Some(node.value.clone());
                    } else {
                        return None;
                    }
                }
            }
        }
        None
    }

    pub fn insert(&mut self, value: T, key: u32) {
        let key_bytes = key.to_be_bytes();
        if self.root.is_null() {
            self.root = Box::into_raw(Box::new(Node::Leaf(LeafNode::new(value, &key_bytes))));
            return;
        }
        let mut depth = 0;
        let mut iter_node = self.root;
        let mut parent_node = &mut self.root as *mut *mut Node<T>;
        let new_leaf = Box::into_raw(Box::new(Node::Leaf(LeafNode::new(
            value.clone(),
            &key_bytes,
        ))));
        match unsafe { &*self.root } {
            Node::N4(_) => println!("N4"),
            Node::N16(_) => println!("N16"),
            Node::Leaf(_) => println!("Leaf"),
        }
        while !iter_node.is_null() {
            match unsafe { &mut *iter_node } {
                Node::N4(node) => {
                    println!("first: {}", depth);
                    let cm = common_prefix(&node.info.partial, &key_bytes);
                    println!("{}, {:?}", node.info.partial_len, key_bytes);
                    if cm != node.info.partial_len {
                        println!(
                            "177: {:?}, {:?}, {}, {}",
                            key_bytes, node.info.partial, depth, cm
                        );
                        let mut new_node = Node4::new(&node.info.partial[..cm]);
                        new_node.add(new_leaf, &key_bytes, depth + cm);
                        new_node.add(iter_node, &node.info.partial, cm);
                        node.info.partial_len -= cm + 1;
                        for i in 0..node.info.partial_len {
                            node.info.partial[i] = node.info.partial[cm + 1 + i];
                        }
                        //let new_node = Node::N4(new_node);
                        unsafe {
                            *parent_node = Box::into_raw(Box::new(Node::N4(new_node)));
                        }
                        break;
                    }
                    depth += cm;
                    parent_node = iter_node as *mut *mut Node<T>;
                    if let Some(n) = self.find_child(unsafe { &*iter_node }, key_bytes[depth]) {
                        iter_node = n;
                    } else {
                        unsafe {
                            (*iter_node).add(new_leaf, &key_bytes, depth);
                        }
                        break;
                    }
                }
                Node::N16(node) => (),
                Node::Leaf(node) => {
                    depth += common_prefix(&node.key, &key_bytes);
                    println!("leaf: {}", depth);
                    if depth == node.key.len() {
                        return;
                    }
                    let mut new_node = Node4::new(&key_bytes[..depth]);
                    //node.key = node.key.to_vec();
                    new_node.add(new_leaf, &key_bytes, depth);
                    println!("iter_node: {:?}", unsafe { &*iter_node });
                    new_node.add(iter_node, &node.key, depth);
                    unsafe {
                        *parent_node = Box::into_raw(Box::new(Node::N4(new_node)));
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_add_and_find() {
        let mut art = Art::<u32>::new();
        let mut data: Vec<[u32; 2]> = vec![];
        let mut rng = rand::thread_rng();

        for _i in 0..100_000 {
            data.push([rng.gen::<u32>(), rng.gen::<u32>()]);
        }

        for elem in &data {
            art.insert(elem[0], elem[1]);
        }

        for elem in &data {
            assert_eq!(elem[0], art.find(elem[1]).unwrap());
        }
    }
}
