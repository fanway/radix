use core::marker::PhantomData;
use std::collections::VecDeque;
use std::ptr;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

trait ArtNode<T: 'static + std::fmt::Debug>: std::fmt::Debug {
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize);
    fn find_child<'a>(&'a mut self, key: u8) -> Option<&'a mut *mut Node<T>>;
    fn delete_child(
        &mut self,
        parent_node: *mut *mut Node<T>,
        ref_node: *mut *mut Node<T>,
        key: u8,
    );
    fn prefix(&self, key: &[u8]) -> usize;
    fn info(&self) -> &Info;
    fn info_mut(&mut self) -> &mut Info;
    fn child_pointers(&self) -> &[*mut Node<T>];
    // Check if we need to split the node, when we have an equal partial prefixes
    // and performs one if needed
    fn split_check(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        parent_node: &mut *mut *mut Node<T>,
    ) -> (bool, Option<&mut *mut Node<T>>) {
        // Number of matched bytes with the current node partial
        let cm = self.prefix(&key_bytes[*depth..]);
        let info = self.info_mut();
        if cm != info.partial_len {
            // Create a new node with the splitted partial to the matter of prefix
            let mut new_node = Node4::new(&info.partial[..cm]);
            // Add a new leaf and the current node as a childs
            new_node.add(new_leaf, &key_bytes, *depth + cm);
            new_node.add(*iter_node, &info.partial, cm);
            info.partial_len -= cm;
            // Split the partial to the matter of suffix
            for i in 0..info.partial_len {
                info.partial[i] = info.partial[cm + i];
            }
            unsafe {
                // Write to the place of the current node the new one
                **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
            }
            return (true, None);
        }
        // If a split is not needed find next child
        *depth += info.partial_len;
        (false, self.find_child(key_bytes[*depth]))
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

// Trait to have a byte representation of the accepted key types
pub trait ArtKey {
    fn bytes(&self) -> Vec<u8>;
}

impl ArtKey for String {
    fn bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

// Because rust doesn't have the size_of of a generic types
// we can't return a generic sized array
// For that purpose we use this macro to generate needed code
macro_rules! doit {
    ($($t:ty)*) => ($(impl ArtKey for $t {
        fn bytes(&self) -> Vec<u8> {
            self.to_be_bytes().to_vec()
        }
    })*)
}
doit! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

// Enum that represents 2 type of nodes
#[derive(Debug)]
enum Node<T> {
    ArtNode(Box<dyn ArtNode<T>>),
    Leaf(LeafNode<T>),
}

// Constant that was introduced in the paper to divide long keys
// into chuncks
const MAX_PREFIX_LEN: usize = 10;

// Struct that contains useful information shared between nodes
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Info {
    // Number of childs in the node
    count: usize,
    // Partial prefix
    partial: [u8; MAX_PREFIX_LEN],
    // Length of the partial prefix
    partial_len: usize,
}

// Node with 4 childs with one to one
// child pointers and keys
#[repr(C)]
#[derive(Debug)]
struct Node4<T> {
    child_pointers: [*mut Node<T>; 4],
    info: Info,
    key: [u8; 4],
}

// Node with 16 childs with one to one
// child pointers and keys
#[repr(C)]
#[derive(Debug)]
struct Node16<T> {
    child_pointers: [*mut Node<T>; 16],
    info: Info,
    key: [u8; 16],
}

// Node with 48 childs
#[repr(C)]
struct Node48<T> {
    child_pointers: [*mut Node<T>; 48],
    // Key is used as a map of bytes
    // key[byte as usize] -> gives on of the 48 pointers
    key: [u8; 256],
    info: Info,
}

// std::fmt::Debug is not implemented for arrays with size >= 32
impl<T> std::fmt::Debug for Node48<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Node48")
            .field("child_pointers", &&self.child_pointers[..])
            .field("key", &&self.key[..])
            .field("info", &self.info)
            .finish()
    }
}

// Node with 256 child, where child_pointers array
// used like a key map
#[repr(C)]
struct Node256<T> {
    child_pointers: [*mut Node<T>; 256],
    info: Info,
}

// std::fmt::Debug is not implemented for arrays with size >= 32
impl<T> std::fmt::Debug for Node256<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Node256")
            .field("child_pointers", &&self.child_pointers[..])
            .field("info", &self.info)
            .finish()
    }
}

// A leaf node which contains a value and a full key
#[repr(C)]
#[derive(Debug)]
struct LeafNode<T> {
    key: Vec<u8>,
    value: T,
}

// Implementation of `Node4`
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

    // New with a copied info header
    fn new_with_info(info: Info) -> Self {
        Self {
            child_pointers: [std::ptr::null_mut(); 4],
            info,
            key: [0; 4],
        }
    }
}

// Implementation of `ArtNode` trait for `Node4`
impl<T: 'static + std::fmt::Debug> ArtNode<T> for Node4<T> {
    fn add(&mut self, node: *mut Node<T>, key: &[u8], depth: usize) {
        let mut i: usize = 0;
        while i < 3 && i < self.info.count {
            if key[depth] < self.key[i] {
                break;
            }
            i += 1;
        }
        // Shift all childs if needed to create space for a new one
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
    fn child_pointers(&self) -> &[*mut Node<T>] {
        &self.child_pointers
    }
    fn prefix(&self, key: &[u8]) -> usize {
        common_prefix(&self.info.partial[..self.info.partial_len], &key)
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        // Condition to continue loop or not
        let mut cont = true;
        // Check for a split and perform split if needed
        let (splitted, n) = self.split_check(key_bytes, depth, iter_node, new_leaf, parent_node);
        if splitted {
            return !splitted;
        }
        if let Some(node) = n {
            *parent_node = node;
            *iter_node = *node;
        } else {
            if self.info.count < 4 {
                self.add(new_leaf, &key_bytes, *depth);
            } else {
                // If we don't have space to insert a new node => expand
                unsafe {
                    let mut new_node = Node16::new_with_info(self.info);
                    // memcpy
                    ptr::copy_nonoverlapping(
                        (&self.key).as_ptr(),
                        (&mut new_node.key).as_mut_ptr(),
                        self.info.count,
                    );
                    // memcpy
                    ptr::copy_nonoverlapping(
                        (&self.child_pointers).as_ptr(),
                        (&mut new_node.child_pointers).as_mut_ptr(),
                        self.info.count,
                    );
                    new_node.add(new_leaf, &key_bytes, *depth);
                    // Free memory for the current node
                    Box::from_raw(*iter_node);
                    **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
                }
            }
            cont = false;
        }
        cont
    }
    fn delete_child(
        &mut self,
        parent_node: *mut *mut Node<T>,
        ref_node: *mut *mut Node<T>,
        _key: u8,
    ) {
        unsafe {
            // Calculating offset in the `child_pointers` to basicly get an index
            let position = ref_node.offset_from((&self.child_pointers).as_ptr());
            // memmove
            ptr::copy(
                (&self.key).as_ptr().offset(position + 1),
                (&mut self.key).as_mut_ptr().offset(position),
                self.info.count - 1 - position as usize,
            );
            // memmove
            ptr::copy(
                (&self.child_pointers).as_ptr().offset(position + 1),
                (&mut self.child_pointers).as_mut_ptr().offset(position),
                self.info.count - 1 - position as usize,
            );
        }
        self.info.count -= 1;
        // If number of childs is equal 1, we want to concat
        // parent and child node together and free the memory
        if self.info.count == 1 {
            let node = self.child_pointers[0];
            if let Node::ArtNode(n) = unsafe { &mut *node } {
                let mut prefix: usize = self.info.partial_len;
                if prefix < MAX_PREFIX_LEN {
                    // Place key-byte to the end of the partial
                    // to later copy it to a leaf
                    self.info.partial[prefix] = self.key[0];
                    prefix += 1;
                }
                let info = n.info_mut();
                unsafe {
                    if prefix < MAX_PREFIX_LEN {
                        // Calculate the remaining prefix
                        let sub_prefix = std::cmp::min(info.partial_len, MAX_PREFIX_LEN - prefix);
                        // Memcpy the remaining prefix to concat it
                        ptr::copy_nonoverlapping(
                            (&info.partial).as_ptr(),
                            (&mut self.info.partial)
                                .as_mut_ptr()
                                .offset(prefix as isize),
                            sub_prefix,
                        );
                        prefix += sub_prefix;
                    }
                    // Memcpy whole partial prefix
                    ptr::copy_nonoverlapping(
                        (&self.info.partial).as_ptr(),
                        (&mut info.partial).as_mut_ptr(),
                        std::cmp::min(prefix, MAX_PREFIX_LEN),
                    );
                    // Because we added key-byte to the end of partial
                    // we have to add 1
                    info.partial_len += self.info.partial_len + 1;
                }
            }
            unsafe {
                // Free the memory
                Box::from_raw(*parent_node);
                *parent_node = node;
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
        // Create a mask with length equal to number
        // of `child_pointers`
        let mask = (1 << self.info.count) - 1;
        unsafe {
            // Compare less than with searched byte
            // for 16 bytes at once
            let cmp = _mm_cmplt_epi8(
                _mm_set1_epi8(key[depth] as i8),
                _mm_loadu_si128((&self.key).as_ptr() as *const __m128i),
            );

            // Apply the mask
            let bitfield = _mm_movemask_epi8(cmp) & mask;
            let i: usize;
            if bitfield > 0 {
                // Trailing zeros represents index
                i = bitfield.trailing_zeros() as usize;
                // Safe memmove (Maybe should make it unsafe to
                // avoid unnecessary bound check
                self.key.copy_within(i..self.info.count, i + 1);
                self.child_pointers.copy_within(i..self.info.count, i + 1);
            } else {
                // If all elements is less than the key, insert to the end
                i = self.info.count;
            }
            // Insert the new node
            self.key[i] = key[depth];
            self.child_pointers[i] = node;
            self.info.count += 1;
        }
    }
    fn find_child<'a>(&'a mut self, key: u8) -> Option<&'a mut *mut Node<T>> {
        let mask = (1 << self.info.count) - 1;
        unsafe {
            // Compare less than with searched byte
            // for 16 bytes at once
            let cmp = _mm_cmpeq_epi8(
                _mm_set1_epi8(key as i8),
                _mm_loadu_si128((&self.key).as_ptr() as *const __m128i),
            );

            // Apply the mask
            let bitfield = _mm_movemask_epi8(cmp) & mask;
            if bitfield != 0 {
                // Return index
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
    fn child_pointers(&self) -> &[*mut Node<T>] {
        &self.child_pointers
    }
    fn prefix(&self, key: &[u8]) -> usize {
        common_prefix(&self.info.partial[..self.info.partial_len], &key)
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        // Condition to continue loop or not
        let mut cont = true;
        // Check for a split and perform split if needed
        let (splitted, n) = self.split_check(key_bytes, depth, iter_node, new_leaf, parent_node);
        if splitted {
            return !splitted;
        }
        if let Some(node) = n {
            *parent_node = node;
            *iter_node = *node;
        } else {
            if self.info.count < 16 {
                self.add(new_leaf, &key_bytes, *depth);
            } else {
                unsafe {
                    // If we don't have space to insert a new node => expand
                    let mut new_node = Node48::new_with_info(self.info);
                    // Memcpy
                    ptr::copy_nonoverlapping(
                        (&self.child_pointers).as_ptr(),
                        (&mut new_node.child_pointers).as_mut_ptr(),
                        self.info.count,
                    );
                    for i in 0..self.info.count {
                        new_node.key[self.key[i] as usize] = i as u8;
                    }
                    new_node.add(new_leaf, &key_bytes, *depth);
                    Box::from_raw(*iter_node);
                    **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
                }
            }
            cont = false;
        }
        cont
    }
    fn delete_child(
        &mut self,
        parent_node: *mut *mut Node<T>,
        ref_node: *mut *mut Node<T>,
        _key: u8,
    ) {
        unsafe {
            // Calculating offset in the `child_pointers` to basicly get an index
            let position = ref_node.offset_from((&self.child_pointers).as_ptr());
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
        // If count == 3 we want to shrink `Node16` to `Node4`
        if self.info.count == 3 {
            let mut new_node = Node4::new_with_info(self.info);
            unsafe {
                ptr::copy_nonoverlapping((&self.key).as_ptr(), (&mut new_node.key).as_mut_ptr(), 4);
                ptr::copy_nonoverlapping(
                    (&self.child_pointers).as_ptr(),
                    (&mut new_node.child_pointers).as_mut_ptr(),
                    4,
                );
                Box::from_raw(*parent_node);
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
        // Add to a free place
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
    fn child_pointers(&self) -> &[*mut Node<T>] {
        &self.child_pointers
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        // Condition to continue loop or not
        let mut cont = true;
        // Check for a split and perform split if needed
        let (splitted, n) = self.split_check(key_bytes, depth, iter_node, new_leaf, parent_node);
        if splitted {
            return !splitted;
        }
        if let Some(node) = n {
            *parent_node = node;
            *iter_node = *node;
        } else {
            if self.info.count < 48 {
                self.add(new_leaf, &key_bytes, *depth);
            } else {
                // If we don't have space to insert a new node => expand
                let mut new_node = Node256::new_with_info(self.info);
                for i in 0..256 {
                    if self.key[i] != 48 {
                        new_node.child_pointers[i] = self.child_pointers[self.key[i] as usize];
                    }
                }
                new_node.add(new_leaf, &key_bytes, *depth);
                unsafe {
                    Box::from_raw(*iter_node);
                    **parent_node = Box::into_raw(Box::new(Node::ArtNode(Box::new(new_node))));
                }
            }
            cont = false;
        }
        cont
    }
    fn delete_child(
        &mut self,
        parent_node: *mut *mut Node<T>,
        _ref_node: *mut *mut Node<T>,
        key: u8,
    ) {
        // Delete child
        let mut position = self.key[key as usize];
        self.key[key as usize] = 48;
        self.child_pointers[position as usize] = ptr::null_mut();
        self.info.count -= 1;

        // If count == 12 we want to shrink `Node48` to `Node16`
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
                Box::from_raw(*parent_node);
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
    fn child_pointers(&self) -> &[*mut Node<T>] {
        &self.child_pointers
    }
    fn prefix(&self, key: &[u8]) -> usize {
        common_prefix(&self.info.partial[..self.info.partial_len], &key)
    }
    fn insert(
        &mut self,
        key_bytes: &[u8],
        depth: &mut usize,
        iter_node: &mut *mut Node<T>,
        new_leaf: *mut Node<T>,
        parent_node: &mut *mut *mut Node<T>,
    ) -> bool {
        // Condition to continue loop or not
        let mut cont = true;
        // Check for a split and perform split if needed
        let (splitted, n) = self.split_check(key_bytes, depth, iter_node, new_leaf, parent_node);
        if splitted {
            return !splitted;
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
    fn delete_child(
        &mut self,
        parent_node: *mut *mut Node<T>,
        _ref_node: *mut *mut Node<T>,
        key: u8,
    ) {
        // Delete child
        self.child_pointers[key as usize] = ptr::null_mut();
        self.info.count -= 1;

        // If count == 35 we wan't to shrink `Node256` to `Node48`
        // (35 is chosen because we don't want to reallocate too much)
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
                Box::from_raw(*parent_node);
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

// Calculate a number of equal bytes in two slices
fn common_prefix(key: &[u8], partial: &[u8]) -> usize {
    key.iter()
        .zip(partial.iter())
        .take_while(|&(a, b)| a == b)
        .count()
}

pub struct Art<K, T: 'static + std::fmt::Debug> {
    root: *mut Node<T>,
    key: PhantomData<K>,
}

// Free all tree recursive
fn free_tree<T: 'static + std::fmt::Debug>(node: *mut Node<T>) {
    if node.is_null() {
        return;
    }
    match unsafe { &*node } {
        Node::ArtNode(n) => {
            let child_pointers = n.child_pointers();
            for ptr in child_pointers.iter() {
                free_tree(*ptr);
            }
        }
        _ => (),
    }
    unsafe {
        Box::from_raw(node);
    }
}

impl<K, T: 'static + std::fmt::Debug> Drop for Art<K, T> {
    fn drop(&mut self) {
        free_tree::<T>(self.root)
    }
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

    // Count a number of nodes in the tree
    pub fn bfs_count(&self) -> usize {
        let mut count = 0;
        if self.root.is_null() {
            return count;
        }
        let mut queue = VecDeque::new();
        queue.push_back(self.root);
        while !queue.is_empty() {
            let node = queue.pop_front().unwrap();
            match unsafe { &*node } {
                Node::ArtNode(n) => {
                    count += 1;
                    let pointers = n.child_pointers();
                    let info = n.info();
                    for i in 0..info.count {
                        queue.push_back(pointers[i]);
                    }
                }
                Node::Leaf(_) => {
                    count += 1;
                }
            }
        }
        count
    }

    // Delete value from the tree
    pub fn delete(&mut self, key: K) {
        let key_bytes = key.bytes();
        let mut ref_node = &mut self.root as *mut *mut Node<T>;
        let mut parent_node = &mut self.root as *mut *mut Node<T>;
        let mut iter_node = self.root;
        let mut depth = 0;
        let mut key = 0;
        while !iter_node.is_null() {
            unsafe {
                println!("iter_node: {:?}, {:?}", *iter_node, key_bytes);
            }
            match unsafe { &mut *iter_node } {
                Node::ArtNode(node) => {
                    depth += node.prefix(&key_bytes[depth..]);
                    // In this case we want last element
                    if depth == key_bytes.len() {
                        depth -= 1;
                    }
                    // Iterate until we hit a leaf or don't find any child
                    if let Some(n) = node.find_child(key_bytes[depth]) {
                        key = key_bytes[depth];
                        parent_node = ref_node;
                        ref_node = n;
                        iter_node = *n;
                    } else {
                        break;
                    }
                }
                Node::Leaf(node) => {
                    depth += common_prefix(&node.key[depth..], &key_bytes[depth..]);
                    if depth == node.key.len() {
                        unsafe {
                            match &mut **parent_node {
                                Node::ArtNode(node) => {
                                    node.delete_child(parent_node, ref_node, key);
                                }
                                // Initial case then parent and child node
                                // might be leaves at the same time
                                Node::Leaf(_) => {
                                    *ref_node = ptr::null_mut();
                                }
                            }
                            Box::from_raw(iter_node);
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
        while !iter_node.is_null() {
            unsafe {
                println!("iter_node: {:?}, {:?}", *iter_node, key.bytes());
            }
            match unsafe { &mut *iter_node } {
                Node::ArtNode(node) => {
                    depth += node.prefix(&key_bytes[depth..]);
                    if depth == key_bytes.len() {
                        depth -= 1;
                    }
                    // Iterate until we hit a leaf or don't find any child
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
                // Either rewrite or split the node
                Node::Leaf(node) => {
                    let cm = depth + common_prefix(&node.key[depth..], &key_bytes[depth..]);
                    println!(
                        "{:?}, {:?}, {:?}",
                        &key_bytes[depth..cm],
                        &key_bytes,
                        &node.key
                    );
                    // Rewrite value of existing node
                    if key_bytes.len() == cm {
                        println!("{:?}, {:?}, {:?}", value, node.value, key);
                        node.value = value;
                        break;
                    }
                    // Split node
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

    #[test]
    fn test_add_and_delete() {
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
            art.delete(key.clone());
        }
        assert_eq!(0, art.bfs_count());
    }
}
