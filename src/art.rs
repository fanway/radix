use core::marker::PhantomData;
use std::ptr;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

trait ArtNode<T: 'static + std::fmt::Debug>: std::fmt::Debug {
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize);
    fn find_child<'a>(&'a mut self, key: u8) -> Option<&'a mut *mut Node<T>>;
    fn delete_child(&mut self, parent_node: *mut *mut Node<T>, key: u8);
    fn prefix(&self, key: &[u8]) -> usize;
    fn info(&self) -> &Info;
    fn info_mut(&mut self) -> &mut Info;
    fn split_check(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        parent_node: &mut *mut *mut Node<T>,
    ) -> (bool, Option<&mut *mut Node<T>>) {
        let cm = self.prefix(&key_bytes[*depth..]);
        let info = self.info_mut();
        if cm != info.partial_len {
            let mut new_node = Node4::new(&info.partial[..cm]);
            new_node.add(new_leaf, &key_bytes, *depth + cm);
            new_node.add(*iter_node, &info.partial, cm);
            info.partial_len -= cm;
            //info.partial.copy_within(0..info.partial_len, cm);
            for i in 0..info.partial_len {
                info.partial[i] = info.partial[cm + i];
            }
            unsafe {
                **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
            }
            return (false, None);
        }
        *depth += info.partial_len;
        (true, self.find_child(key_bytes[*depth]))
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        parent_node: &mut *mut *mut Node<T>,
    ) -> bool;
}

pub trait ArtKey {
    fn bytes(&self) -> Vec<u8>;
}

impl ArtKey for String {
    fn bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

macro_rules! doit {
    ($($t:ty)*) => ($(impl ArtKey for $t {
        fn bytes(&self) -> Vec<u8> {
            self.to_be_bytes().to_vec()
        }
    })*)
}
doit! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

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

impl<T> Node4<T> {
    fn new(prefix: &[u8]) -> Self {
        let min = std::cmp::min(MAX_PREFIX_LEN, prefix.len());
        let mut partial = [0; MAX_PREFIX_LEN];
        partial[..min].copy_from_slice(&prefix[..min]);
        Self {
            child_pointers: [std::ptr::null_mut(); 4],
            info: Info {
                count: 0,
                partial,
                partial_len: min,
            },
            key: [0; 4],
        }
    }

    fn new_with_info(info: Info) -> Self {
        Self {
            child_pointers: [std::ptr::null_mut(); 4],
            info,
            key: [0; 4],
        }
    }
}

impl<T: 'static + std::fmt::Debug> ArtNode<T> for Node4<T> {
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
    fn find_child<'a>(&'a mut self, key: u8) -> Option<&'a mut *mut Node<T>> {
        for i in 0..self.info.count as usize {
            if key == self.key[i] {
                return Some(&mut self.child_pointers[i]);
            }
        }
        None
    }
    fn info(&self) -> &Info {
        &self.info
    }
    fn info_mut(&mut self) -> &mut Info {
        &mut self.info
    }
    fn prefix(&self, key: &[u8]) -> usize {
        common_prefix(&self.info.partial[..self.info.partial_len], &key)
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        mut iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        mut parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        let mut cont = true;
        let (splitted, n) =
            self.split_check(key_bytes, depth, &mut iter_node, new_leaf, &mut parent_node);
        if !splitted {
            return splitted;
        }
        if let Some(node) = n {
            *parent_node = node;
            *iter_node = *node;
        } else {
            if self.info.count < 4 {
                self.add(new_leaf, &key_bytes, *depth);
            } else {
                unsafe {
                    let mut new_node = Node16::new_with_info(self.info);
                    ptr::copy_nonoverlapping(
                        (&self.key).as_ptr(),
                        (&mut new_node.key).as_mut_ptr(),
                        self.info.count,
                    );
                    ptr::copy_nonoverlapping(
                        (&self.child_pointers).as_ptr(),
                        (&mut new_node.child_pointers).as_mut_ptr(),
                        self.info.count,
                    );
                    new_node.add(new_leaf, &key_bytes, *depth);
                    ptr::drop_in_place(*iter_node);
                    **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
                }
            }
            cont = false;
        }
        cont
    }
    fn delete_child(&mut self, parent_node: *mut *mut Node<T>, _key: u8) {
        unsafe {
            let position = parent_node.offset_from((&self.child_pointers).as_ptr());
            ptr::copy(
                (&self.key).as_ptr().offset(position + 1),
                (&mut self.key).as_mut_ptr().offset(position),
                self.info.count - 1 - position as usize,
            );
            ptr::copy(
                (&self.child_pointers).as_ptr().offset(position + 1),
                (&mut self.child_pointers).as_mut_ptr().offset(position),
                self.info.count - 1 - position as usize,
            );
        }
        self.info.count -= 1;
        if self.info.count == 1 {
            let node = self.child_pointers[0];
            if let Node::ArtNode(n) = unsafe { &mut *node } {
                let mut prefix: usize = self.info.partial_len;
                if prefix < MAX_PREFIX_LEN {
                    self.info.partial[prefix] = self.key[0];
                    prefix += 1;
                }
                let info = n.info_mut();
                unsafe {
                    if prefix < MAX_PREFIX_LEN {
                        let sub_prefix = std::cmp::min(info.partial_len, MAX_PREFIX_LEN - prefix);
                        ptr::copy_nonoverlapping(
                            (&info.partial).as_ptr(),
                            (&mut self.info.partial)
                                .as_mut_ptr()
                                .offset(prefix as isize),
                            sub_prefix,
                        );
                        prefix += sub_prefix;
                        ptr::copy_nonoverlapping(
                            (&info.partial).as_ptr(),
                            (&mut self.info.partial).as_mut_ptr(),
                            std::cmp::min(prefix, MAX_PREFIX_LEN),
                        );
                        info.partial_len += prefix + 1;
                    }
                    *parent_node = node;
                }
            }
        }
    }
}

impl<T> Node16<T> {
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
}

impl<T: 'static + std::fmt::Debug> ArtNode<T> for Node16<T> {
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
            self.key[i] = key[depth];
            self.child_pointers[i] = node;
            println!("{}, {}, {:?}", i, key[depth], self.key);
            self.info.count += 1;
        }
    }
    fn find_child<'a>(&'a mut self, key: u8) -> Option<&'a mut *mut Node<T>> {
        let mask = (1 << self.info.count) - 1;
        unsafe {
            let cmp = _mm_cmpeq_epi8(
                _mm_set1_epi8(key as i8),
                _mm_loadu_si128((&self.key).as_ptr() as *const __m128i),
            );

            let bitfield = _mm_movemask_epi8(cmp) & mask;
            if bitfield != 0 {
                let i = bitfield.trailing_zeros() as usize;
                return Some(&mut self.child_pointers[i]);
            }
            return None;
        }
    }
    fn info(&self) -> &Info {
        &self.info
    }
    fn info_mut(&mut self) -> &mut Info {
        &mut self.info
    }
    fn prefix(&self, key: &[u8]) -> usize {
        common_prefix(&self.info.partial[..self.info.partial_len], &key)
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        mut iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        mut parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        let mut cont = true;
        let (splitted, n) =
            self.split_check(key_bytes, depth, &mut iter_node, new_leaf, &mut parent_node);
        if !splitted {
            return splitted;
        }
        if let Some(node) = n {
            *parent_node = node;
            *iter_node = *node;
        } else {
            if self.info.count < 16 {
                self.add(new_leaf, &key_bytes, *depth);
            } else {
                unsafe {
                    let mut new_node = Node48::new_with_info(self.info);
                    ptr::copy_nonoverlapping(
                        (&self.child_pointers).as_ptr(),
                        (&mut new_node.child_pointers).as_mut_ptr(),
                        self.info.count,
                    );
                    for i in 0..self.info.count {
                        new_node.key[self.key[i] as usize] = i as u8;
                    }
                    new_node.add(new_leaf, &key_bytes, *depth);
                    ptr::drop_in_place(*iter_node);
                    **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
                }
            }
            cont = false;
        }
        cont
    }
    fn delete_child(&mut self, parent_node: *mut *mut Node<T>, _key: u8) {
        unsafe {
            let position = parent_node.offset_from((&self.child_pointers).as_ptr());
            ptr::copy(
                (&self.key).as_ptr().offset(position + 1),
                (&mut self.key).as_mut_ptr().offset(position),
                self.info.count - 1 - position as usize,
            );
            ptr::copy(
                (&self.child_pointers).as_ptr().offset(position + 1),
                (&mut self.child_pointers).as_mut_ptr().offset(position),
                self.info.count - 1 - position as usize,
            );
        }
        self.info.count -= 1;
        if self.info.count == 3 {
            let mut new_node = Node4::new_with_info(self.info);
            unsafe {
                ptr::copy_nonoverlapping((&self.key).as_ptr(), (&mut new_node.key).as_mut_ptr(), 4);
                ptr::copy_nonoverlapping(
                    (&self.child_pointers).as_ptr(),
                    (&mut new_node.child_pointers).as_mut_ptr(),
                    4,
                );
                *parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
            }
        }
    }
}

impl<T> Node48<T> {
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
}

impl<T: 'static + std::fmt::Debug> ArtNode<T> for Node48<T> {
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        let mut i = 0;
        while !self.child_pointers[i].is_null() {
            i += 1;
        }
        self.child_pointers[i] = node;
        self.key[key[depth] as usize] = i as u8;
        self.info.count += 1;
    }
    fn find_child<'a>(&'a mut self, key: u8) -> Option<&'a mut *mut Node<T>> {
        if self.key[key as usize] != 48 {
            return Some(&mut self.child_pointers[self.key[key as usize] as usize]);
        }
        None
    }
    fn prefix(&self, key: &[u8]) -> usize {
        common_prefix(&self.info.partial[..self.info.partial_len], &key)
    }
    fn info(&self) -> &Info {
        &self.info
    }
    fn info_mut(&mut self) -> &mut Info {
        &mut self.info
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        mut iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        mut parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        let mut cont = true;
        let (splitted, n) =
            self.split_check(key_bytes, depth, &mut iter_node, new_leaf, &mut parent_node);
        if !splitted {
            return splitted;
        }
        if let Some(node) = n {
            *parent_node = node;
            *iter_node = *node;
        } else {
            if self.info.count < 48 {
                self.add(new_leaf, &key_bytes, *depth);
            } else {
                let mut new_node = Node256::new_with_info(self.info);
                for i in 0..256 {
                    if self.key[i] != 48 {
                        new_node.child_pointers[i] = self.child_pointers[self.key[i] as usize];
                    }
                }
                new_node.add(new_leaf, &key_bytes, *depth);
                unsafe {
                    ptr::drop_in_place(*iter_node);
                    **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
                }
            }
            cont = false;
        }
        cont
    }
    fn delete_child(&mut self, parent_node: *mut *mut Node<T>, key: u8) {
        let mut position = self.key[key as usize];
        self.key[key as usize] = 48;
        self.child_pointers[position as usize] = ptr::null_mut();
        self.info.count -= 1;

        if self.info.count == 12 {
            let mut new_node = Node16::new_with_info(self.info);
            let mut count = 0;
            for i in 0..256 {
                position = self.key[i];
                if position != 48 {
                    new_node.key[count] = i as u8;
                    new_node.child_pointers[count] = self.child_pointers[position as usize];
                    count += 1;
                }
            }
            unsafe {
                *parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
            }
        }
    }
}

impl<T> Node256<T> {
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
}

impl<T: 'static + std::fmt::Debug> ArtNode<T> for Node256<T> {
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        self.child_pointers[key[depth] as usize] = node;
        self.info.count += 1;
    }
    fn find_child<'a>(&'a mut self, key: u8) -> Option<&'a mut *mut Node<T>> {
        if !self.child_pointers[key as usize].is_null() {
            return Some(&mut self.child_pointers[key as usize]);
        }
        None
    }
    fn info(&self) -> &Info {
        &self.info
    }
    fn info_mut(&mut self) -> &mut Info {
        &mut self.info
    }
    fn prefix(&self, key: &[u8]) -> usize {
        common_prefix(&self.info.partial[..self.info.partial_len], &key)
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        mut iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        mut parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        let mut cont = true;
        let (splitted, n) =
            self.split_check(key_bytes, depth, &mut iter_node, new_leaf, &mut parent_node);
        if !splitted {
            return splitted;
        }
        if let Some(node) = n {
            *parent_node = node;
            *iter_node = *node;
        } else {
            self.add(new_leaf, &key_bytes, *depth);
            cont = false;
        }
        cont
    }
    fn delete_child(&mut self, parent_node: *mut *mut Node<T>, key: u8) {
        self.child_pointers[key as usize] = ptr::null_mut();
        self.info.count -= 1;

        if self.info.count == 35 {
            let mut new_node = Node48::new_with_info(self.info);
            let mut position = 0;
            for i in 0..256 {
                if !self.child_pointers[i].is_null() {
                    new_node.child_pointers[position] = self.child_pointers[i];
                    new_node.key[i] = position as u8;
                    position += 1;
                }
            }
            unsafe {
                *parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
            }
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

pub struct Art<K, T> {
    root: *mut Node<T>,
    key: PhantomData<K>,
}

impl<K, T> Art<K, T>
where
    K: ArtKey + std::marker::Sized + std::fmt::Debug,
    T: 'static + Clone + std::fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            root: std::ptr::null_mut(),
            key: PhantomData,
        }
    }

    pub fn delete(&mut self, key: K) {
        let key_bytes = key.bytes();
        let mut ref_node = &mut self.root as *mut *mut Node<T>;
        let mut parent_node = self.root;
        let mut iter_node = self.root;
        let mut depth = 0;
        let mut key = 0;
        while !iter_node.is_null() {
            match unsafe { &mut *iter_node } {
                Node::ArtNode(node) => {
                    depth += node.prefix(&key_bytes[depth..]);
                    if let Some(n) = node.find_child(key_bytes[depth]) {
                        key = key_bytes[depth];
                        ref_node = n;
                        parent_node = iter_node;
                        iter_node = *n;
                    } else {
                        break;
                    }
                }
                Node::Leaf(node) => {
                    depth += common_prefix(&node.key[depth..], &key_bytes[depth..]);
                    if depth == node.key.len() {
                        unsafe {
                            match &mut *parent_node {
                                Node::ArtNode(node) => node.delete_child(ref_node, key),
                                Node::Leaf(_) => (),
                            }
                            ptr::drop_in_place(iter_node);
                            iter_node = ptr::null_mut();
                        }
                    }
                    break;
                }
            }
        }
    }

    pub fn find(&self, key: K) -> Option<&T> {
        let mut iter_node = self.root;
        let key_bytes = key.bytes();
        let mut depth = 0;
        println!("----------------------------");
        while !iter_node.is_null() {
            unsafe {
                println!("iter_node: {:?}, {:?}", *iter_node, key.bytes());
            }
            match unsafe { &mut *iter_node } {
                Node::ArtNode(node) => {
                    depth += node.prefix(&key_bytes[depth..]);
                    if let Some(n) = node.find_child(key_bytes[depth]) {
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

    pub fn insert(&mut self, key: K, value: T) {
        let key_bytes = key.bytes();
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
        while !iter_node.is_null() {
            match unsafe { &mut *iter_node } {
                Node::ArtNode(node) => {
                    if !node.insert(
                        &key_bytes,
                        &mut depth,
                        &mut iter_node,
                        new_leaf,
                        &mut parent_node,
                    ) {
                        break;
                    }
                }
                Node::Leaf(node) => {
                    let cm = depth + common_prefix(&node.key[depth..], &key_bytes[depth..]);
                    println!(
                        "{:?}, {:?}, {:?}",
                        &key_bytes[depth..cm],
                        &key_bytes,
                        &node.key
                    );
                    if key_bytes.len() == cm {
                        println!("{:?}, {:?}, {:?}", value, node.value, key);
                        node.value = value;
                        break;
                    }
                    let mut new_node = Node4::new(&key_bytes[depth..cm]);
                    //node.key = node.key.to_vec();
                    new_node.add(new_leaf, &key_bytes, cm);
                    new_node.add(iter_node, &node.key, cm);
                    unsafe {
                        *parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
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
    use rand::Rng;

    #[test]
    fn test_add_and_find() {
        let mut art = Art::<u32, u32>::new();
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
