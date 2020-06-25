use rand::Rng;
use std::ptr;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

trait ArtNode<T>: std::fmt::Debug {
    fn new(&self, prefix: &[u8]) -> Box<dyn ArtNode<T>>;
    fn new_with_info(&self, info: Info) -> Box<dyn ArtNode<T>>;
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize);
    fn find_child<'a>(&self, node: &'a mut Node<T>, key: u8) -> Option<&'a mut *mut Node<T>>;
}

#[derive(Debug)]
enum Node<T> {
    ArtNode(Box<dyn ArtNode<T>>),
    Leaf(LeafNode<T>),
}

const MAX_PREFIX_LEN: usize = 10;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Info {
    count: usize,
    partial: [u8; MAX_PREFIX_LEN],
    partial_len: usize,
}

#[repr(C)]
#[derive(Debug)]
struct Node4<T> {
    child_pointers: [*mut Node<T>; 4],
    info: Info,
    key: [u8; 4],
}

#[repr(C)]
#[derive(Debug)]
struct Node16<T> {
    child_pointers: [*mut Node<T>; 16],
    info: Info,
    key: [u8; 16],
}

#[repr(C)]
struct Node48<T> {
    child_pointers: [*mut Node<T>; 48],
    key: [u8; 256],
    info: Info,
}

impl<T> std::fmt::Debug for Node48<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Node48")
            .field("child_pointers", &&self.child_pointers[..])
            .field("key", &&self.key[..])
            .field("info", &self.info)
            .finish()
    }
}

#[repr(C)]
struct Node256<T> {
    child_pointers: [*mut Node<T>; 256],
    info: Info,
}

impl<T> std::fmt::Debug for Node256<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Node256")
            .field("child_pointers", &&self.child_pointers[..])
            .field("info", &self.info)
            .finish()
    }
}

#[repr(C)]
#[derive(Debug)]
struct LeafNode<T> {
    key: Vec<u8>,
    value: T,
}

fn transform(value: u32) -> [u8; 4] {
    value.to_be_bytes()
}

impl<T: std::fmt::Debug> ArtNode<T> for Node4<T> {
    fn new(&self, prefix: &[u8]) -> Box<dyn ArtNode<T>> {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(&prefix[..min]);
        Box::new(Self {
            child_pointers: [std::ptr::null_mut(); 4],
            info: Info {
                count: 0,
                partial,
                partial_len: min,
            },
            key: [0; 4],
        })
    }

    fn new_with_info(&self, info: Info) -> Box<dyn ArtNode<T>> {
        Box::new(Self {
            child_pointers: [std::ptr::null_mut(); 4],
            info,
            key: [0; 4],
        })
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
            self.key.copy_within(i..self.info.count, i + 1);
            self.child_pointers.copy_within(i..self.info.count, i + 1);
        }
        self.info.count += 1;
        self.key[i] = key[depth];
        self.child_pointers[i] = node;
    }
}

impl<T: std::fmt::Debug> Node16<T> {
    fn new(prefix: &[u8]) -> Self {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(&prefix[..min]);
        Self {
            child_pointers: [std::ptr::null_mut(); 16],
            info: Info {
                count: 0,
                partial,
                partial_len: min,
            },
            key: [0; 16],
        }
    }

    fn new_with_info(info: Info) -> Self {
        Self {
            child_pointers: [std::ptr::null_mut(); 16],
            info,
            key: [0; 16],
        }
    }

    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        let mask = (1 << self.info.count) - 1;
        unsafe {
            let cmp = _mm_cmplt_epi8(
                _mm_set1_epi8(key[depth] as i8),
                _mm_loadu_si128((&self.key).as_ptr() as *const __m128i),
            );

            let bitfield = _mm_movemask_epi8(cmp) & mask;
            let i: usize;
            if bitfield > 0 {
                i = bitfield.trailing_zeros() as usize;
                self.key.copy_within(i..self.info.count, i + 1);
                self.child_pointers.copy_within(i..self.info.count, i + 1);
            } else {
                i = self.info.count;
            }
            println!("{}, {}, {:?}", i, key[depth], self.key);
            self.key[i] = key[depth];
            self.child_pointers[i] = node;
            self.info.count += 1;
        }
    }
}

impl<T: std::fmt::Debug> Node48<T> {
    fn new(prefix: &[u8]) -> Self {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(&prefix[..min]);
        Self {
            child_pointers: [std::ptr::null_mut(); 48],
            info: Info {
                count: 0,
                partial,
                partial_len: min,
            },
            key: [48; 256],
        }
    }

    fn new_with_info(info: Info) -> Self {
        Self {
            child_pointers: [std::ptr::null_mut(); 48],
            info,
            key: [48; 256],
        }
    }

    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        let mut i = 0;
        while !self.child_pointers[i].is_null() {
            i += 1;
        }
        self.child_pointers[i] = node;
        self.key[key[depth] as usize] = i as u8;
        self.info.count += 1;
    }
}
impl<T: std::fmt::Debug> Node256<T> {
    fn new(prefix: &[u8]) -> Self {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(&prefix[..min]);
        Self {
            child_pointers: [std::ptr::null_mut(); 256],
            info: Info {
                count: 0,
                partial,
                partial_len: min,
            },
        }
    }

    fn new_with_info(info: Info) -> Self {
        Self {
            child_pointers: [std::ptr::null_mut(); 256],
            info,
        }
    }

    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        self.child_pointers[key[depth] as usize] = node;
        self.info.count += 1;
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

    fn find_child<'a>(&self, node: &'a mut Node<T>, key: u8) -> Option<&'a mut *mut Node<T>> {
        match node {
            Node::N4(node) => {
                for i in 0..node.info.count as usize {
                    if key == node.key[i] {
                        return Some(&mut node.child_pointers[i]);
                    }
                }
            }
            Node::N16(node) => {
                let mask = (1 << node.info.count) - 1;
                unsafe {
                    let cmp = _mm_cmpeq_epi8(
                        _mm_set1_epi8(key as i8),
                        _mm_loadu_si128((&node.key).as_ptr() as *const __m128i),
                    );

                    let bitfield = _mm_movemask_epi8(cmp) & mask;
                    if bitfield != 0 {
                        let i = bitfield.trailing_zeros() as usize;
                        return Some(&mut node.child_pointers[i]);
                    }
                    return None;
                }
            }
            Node::N48(node) => {
                if node.key[key as usize] != 48 {
                    return Some(&mut node.child_pointers[node.key[key as usize] as usize]);
                }
                return None;
            }
            Node::N256(node) => {
                if !node.child_pointers[key as usize].is_null() {
                    return Some(&mut node.child_pointers[key as usize]);
                }
                return None;
            }
            Node::Leaf(_) => (),
        }
        None
    }

    pub fn find(&self, key: u32) -> Option<&T> {
        let mut iter_node = self.root;
        let key_bytes = key.to_be_bytes();
        let mut depth = 0;
        println!("----------------------------");
        while !iter_node.is_null() {
            unsafe {
                println!("iter_node: {:?}, {:?}", *iter_node, key.to_be_bytes());
            }
            match unsafe { &*iter_node } {
                Node::N4(node) => {
                    depth += common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        iter_node = *n;
                    } else {
                        break;
                    }
                }
                Node::N16(node) => {
                    depth += common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        iter_node = *n;
                    } else {
                        break;
                    }
                }
                Node::N48(node) => {
                    depth += common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        iter_node = *n;
                    } else {
                        break;
                    }
                }
                Node::N256(node) => {
                    depth += common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        iter_node = *n;
                    } else {
                        break;
                    }
                }
                Node::Leaf(node) => {
                    depth += common_prefix(&node.key[depth..], &key_bytes[depth..]);
                    if depth == node.key.len() {
                        return Some(&node.value);
                    } else {
                        return None;
                    }
                }
            }
        }
        None
    }

    pub fn insert(&mut self, key: u32, value: T) {
        let key_bytes = key.to_be_bytes();
        if self.root.is_null() {
            self.root = Box::into_raw(Box::new(Node::Leaf(LeafNode::new(value, &key_bytes))));
            return;
        }
        let mut depth = 0;
        let mut iter_node = self.root;
        //unsafe {
        //    println!("{:?}", *self.root);
        //}
        let mut parent_node = &mut self.root as *mut *mut Node<T>;
        let new_leaf = Box::into_raw(Box::new(Node::Leaf(LeafNode::new(
            value.clone(),
            &key_bytes,
        ))));
        while !iter_node.is_null() {
            match unsafe { &mut *iter_node } {
                Node::N4(node) => {
                    let cm = common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if cm != node.info.partial_len {
                        let mut new_node = Node4::new(&node.info.partial[..cm]);
                        new_node.add(new_leaf, &key_bytes, depth + cm);
                        new_node.add(iter_node, &node.info.partial, cm);
                        node.info.partial_len -= cm;
                        for i in 0..node.info.partial_len {
                            node.info.partial[i] = node.info.partial[cm + i];
                        }
                        unsafe {
                            *parent_node = Box::into_raw(Box::new(Node::N4(new_node)));
                        }
                        break;
                    }
                    depth += node.info.partial_len;
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        parent_node = n;
                        iter_node = *n;
                    } else {
                        if node.info.count < 4 {
                            node.add(new_leaf, &key_bytes, depth);
                        } else {
                            unsafe {
                                let mut new_node = Node16::new_with_info(node.info);
                                ptr::copy_nonoverlapping(
                                    (&node.key).as_ptr(),
                                    (&mut new_node.key).as_mut_ptr(),
                                    node.info.count,
                                );
                                ptr::copy_nonoverlapping(
                                    (&node.child_pointers).as_ptr(),
                                    (&mut new_node.child_pointers).as_mut_ptr(),
                                    node.info.count,
                                );
                                new_node.add(new_leaf, &key_bytes, depth);
                                ptr::drop_in_place(iter_node);
                                *parent_node = Box::into_raw(Box::new(Node::N16(new_node)));
                            }
                        }
                        break;
                    }
                }
                Node::N16(node) => {
                    let cm = common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if cm != node.info.partial_len {
                        println!("{}", cm);
                        let mut new_node = Node4::new(&node.info.partial[..cm]);
                        new_node.add(new_leaf, &key_bytes, depth + cm);
                        new_node.add(iter_node, &node.info.partial, cm);
                        node.info.partial_len -= cm;
                        for i in 0..node.info.partial_len {
                            node.info.partial[i] = node.info.partial[cm + i];
                        }
                        unsafe {
                            *parent_node = Box::into_raw(Box::new(Node::N4(new_node)));
                        }
                        break;
                    }
                    depth += node.info.partial_len;
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        parent_node = n;
                        iter_node = *n;
                    } else {
                        if node.info.count < 16 {
                            node.add(new_leaf, &key_bytes, depth);
                        } else {
                            unsafe {
                                let mut new_node = Node48::new_with_info(node.info);
                                ptr::copy_nonoverlapping(
                                    (&node.child_pointers).as_ptr(),
                                    (&mut new_node.child_pointers).as_mut_ptr(),
                                    node.info.count,
                                );
                                for i in 0..node.info.count {
                                    new_node.key[node.key[i] as usize] = i as u8;
                                }
                                new_node.add(new_leaf, &key_bytes, depth);
                                ptr::drop_in_place(iter_node);
                                *parent_node = Box::into_raw(Box::new(Node::N48(new_node)));
                            }
                        }
                        break;
                    }
                }
                Node::N48(node) => {
                    let cm = common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if cm != node.info.partial_len {
                        let mut new_node = Node4::new(&node.info.partial[..cm]);
                        new_node.add(new_leaf, &key_bytes, depth + cm);
                        new_node.add(iter_node, &node.info.partial, cm);
                        node.info.partial_len -= cm;
                        for i in 0..node.info.partial_len {
                            node.info.partial[i] = node.info.partial[cm + i];
                        }
                        unsafe {
                            *parent_node = Box::into_raw(Box::new(Node::N4(new_node)));
                        }
                        break;
                    }
                    depth += node.info.partial_len;
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        parent_node = n;
                        iter_node = *n;
                    } else {
                        if node.info.count < 48 {
                            node.add(new_leaf, &key_bytes, depth);
                        } else {
                            let mut new_node = Node256::new_with_info(node.info);
                            for i in 0..256 {
                                if node.key[i] != 48 {
                                    new_node.child_pointers[i] =
                                        node.child_pointers[node.key[i] as usize];
                                }
                            }
                            new_node.add(new_leaf, &key_bytes, depth);
                            unsafe {
                                ptr::drop_in_place(iter_node);
                                *parent_node = Box::into_raw(Box::new(Node::N256(new_node)));
                            }
                        }
                        break;
                    }
                }
                Node::N256(node) => {
                    let cm = common_prefix(
                        &node.info.partial[..node.info.partial_len],
                        &key_bytes[depth..],
                    );
                    if cm != node.info.partial_len {
                        let mut new_node = Node4::new(&node.info.partial[..cm]);
                        new_node.add(new_leaf, &key_bytes, depth + cm);
                        new_node.add(iter_node, &node.info.partial, cm);
                        println!("{}, {:?}", cm + 1, &key_bytes);
                        node.info.partial_len -= cm;
                        for i in 0..node.info.partial_len {
                            node.info.partial[i] = node.info.partial[cm + i];
                        }
                        unsafe {
                            ptr::drop_in_place(iter_node);
                            *parent_node = Box::into_raw(Box::new(Node::N4(new_node)));
                        }
                        break;
                    }
                    depth += node.info.partial_len;
                    if let Some(n) = self.find_child(unsafe { &mut *iter_node }, key_bytes[depth]) {
                        parent_node = n;
                        iter_node = *n;
                    } else {
                        node.add(new_leaf, &key_bytes, depth);
                        break;
                    }
                }

                Node::Leaf(node) => {
                    let cm = depth + common_prefix(&node.key[depth..], &key_bytes[depth..]);
                    let len = node.key.len();
                    println!(
                        "{:?}, {:?}, {:?}",
                        &key_bytes[depth..cm],
                        &key_bytes,
                        &node.key
                    );
                    if key_bytes.len() == cm {
                        println!("{:?}, {:?}, {:?}", value, node.value, key);
                        node.value = value;
                        panic!();
                        break;
                    }
                    let mut new_node = Node4::new(&key_bytes[depth..cm]);
                    //node.key = node.key.to_vec();
                    new_node.add(new_leaf, &key_bytes, cm);
                    new_node.add(iter_node, &node.key, cm);
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
        let mut data = std::collections::HashMap::new();
        let mut rng = rand::thread_rng();

        for _i in 0..100_000 {
            data.insert(rng.gen::<u32>(), rng.gen::<u32>());
        }

        for (key, val) in &data {
            art.insert(key.clone(), val.clone());
        }

        for (key, val) in &data {
            assert_eq!(val, art.find(key.clone()).unwrap());
        }
    }
}
