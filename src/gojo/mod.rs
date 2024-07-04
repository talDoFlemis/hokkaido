use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::marker;
use std::mem;
use std::ops::Index;
use std::ptr;

const MAX_MODS: usize = 6;
const MAX_OPS: usize = 20;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Color {
    Red,
    Black,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ModData<K: Ord + Clone, V: Clone> {
    Parent(NodePtr<K, V>),
    Left(NodePtr<K, V>),
    Right(NodePtr<K, V>),
    Col(Color),
}

struct Mod<K: Ord + Clone, V: Clone> {
    data: ModData<K, V>,
    version: usize,
}

struct GojoNode<K: Ord + Clone, V: Clone> {
    color: Color,
    left: NodePtr<K, V>,
    right: NodePtr<K, V>,
    parent: NodePtr<K, V>,
    bk_ptr_left: NodePtr<K, V>,
    bk_ptr_right: NodePtr<K, V>,
    bk_ptr_parent: NodePtr<K, V>,
    key: K,
    value: V,
    mods: Vec<Mod<K, V>>,
}

impl<K: Ord + Clone, V: Clone> GojoNode<K, V> {
    fn clone_with_latest_mods(&self) -> Self {
        let bk_ptr_left = self.bk_ptr_left;
        let bk_ptr_right = self.bk_ptr_right;
        let bk_ptr_parent = self.bk_ptr_parent;
        let key = self.key.clone();
        let value = self.value.clone();
        let mods = Vec::with_capacity(MAX_MODS);

        let mut color = self.color;
        let mut left = self.left;
        let mut right = self.right;
        let mut parent = self.parent;

        for m in self.mods.iter() {
            match m.data {
                ModData::Parent(p) => parent = p,
                ModData::Left(l) => left = l,
                ModData::Right(r) => right = r,
                ModData::Col(c) => color = c,
            }
        }
        Self {
            color,
            key,
            value,
            mods,
            left,
            right,
            parent,
            bk_ptr_right,
            bk_ptr_left,
            bk_ptr_parent,
        }
    }
}

impl<K: Ord + Clone, V: Clone> GojoNode<K, V> {
    fn pair(self) -> (K, V) {
        (self.key, self.value)
    }
}

impl<K, V> Debug for GojoNode<K, V>
where
    K: Ord + Debug + Clone,
    V: Debug + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "k:{:?} v:{:?} c:{:?}", self.key, self.value, self.color)
    }
}

#[derive(Debug)]
struct NodePtr<K: Ord + Clone, V: Clone>(*mut GojoNode<K, V>);

impl<K: Ord + Clone, V: Clone> Clone for NodePtr<K, V> {
    fn clone(&self) -> NodePtr<K, V> {
        *self
    }
}

impl<K: Ord + Clone, V: Clone> Copy for NodePtr<K, V> {}

impl<K: Ord + Clone, V: Clone> Ord for NodePtr<K, V> {
    fn cmp(&self, other: &NodePtr<K, V>) -> Ordering {
        unsafe { (*self.0).key.cmp(&(*other.0).key) }
    }
}

impl<K: Ord + Clone, V: Clone> PartialOrd for NodePtr<K, V> {
    fn partial_cmp(&self, other: &NodePtr<K, V>) -> Option<Ordering> {
        unsafe { Some((*self.0).key.cmp(&(*other.0).key)) }
    }

    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less))
    }

    fn le(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Less | Ordering::Equal)
        )
    }

    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater))
    }

    fn ge(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Greater | Ordering::Equal)
        )
    }
}

impl<K: Ord + Clone, V: Clone> PartialEq for NodePtr<K, V> {
    fn eq(&self, other: &NodePtr<K, V>) -> bool {
        self.0 == other.0
    }
}

impl<K: Ord + Clone, V: Clone> Eq for NodePtr<K, V> {}

impl<K: Ord + Clone, V: Clone> NodePtr<K, V> {
    fn new(k: K, v: V) -> NodePtr<K, V> {
        let node = GojoNode {
            color: Color::Black,
            left: NodePtr::null(),
            right: NodePtr::null(),
            parent: NodePtr::null(),
            bk_ptr_left: NodePtr::null(),
            bk_ptr_right: NodePtr::null(),
            bk_ptr_parent: NodePtr::null(),
            key: k,
            value: v,
            mods: Vec::with_capacity(MAX_MODS),
        };
        NodePtr(Box::into_raw(Box::new(node)))
    }

    fn set_color(&mut self, color: Color) {
        if self.is_null() {
            return;
        }
        unsafe {
            (*self.0).color = color;
        }
    }

    fn set_red_color(&mut self) {
        self.set_color(Color::Red);
    }

    fn set_black_color(&mut self) {
        self.set_color(Color::Black);
    }

    fn get_color(&self, version: usize) -> Color {
        if self.is_null() {
            return Color::Black;
        }
        unsafe { (*self.0).color }
    }

    fn is_red_color(&self, version: usize) -> bool {
        if self.is_null() {
            return false;
        }
        unsafe { (*self.0).color == Color::Red }
    }

    fn is_black_color(&self, version: usize) -> bool {
        if self.is_null() {
            return true;
        }
        unsafe { (*self.0).color == Color::Black }
    }

    fn is_left_child(&self, version: usize) -> bool {
        self.parent(version).left(version) == *self
    }

    fn is_right_child(&self, version: usize) -> bool {
        self.parent(version).right(version) == *self
    }

    fn min_node(self, version: usize) -> NodePtr<K, V> {
        let mut temp = self;
        while !temp.left(version).is_null() {
            temp = temp.left(version);
        }
        temp
    }

    fn max_node(self, version: usize) -> NodePtr<K, V> {
        let mut temp = self;
        while !temp.right(version).is_null() {
            temp = temp.right(version);
        }
        temp
    }

    fn next(self, version: usize) -> NodePtr<K, V> {
        if !self.right(version).is_null() {
            self.right(version).min_node(version)
        } else {
            let mut temp = self;
            loop {
                if temp.parent(version).is_null() {
                    return NodePtr::null();
                }
                if temp.is_left_child(version) {
                    return temp.parent(version);
                }
                temp = temp.parent(version);
            }
        }
    }

    fn prev(self, version: usize) -> NodePtr<K, V> {
        if !self.left(version).is_null() {
            self.left(version).max_node(version)
        } else {
            let mut temp = self;
            loop {
                if temp.parent(version).is_null() {
                    return NodePtr::null();
                }
                if temp.is_right_child(version) {
                    return temp.parent(version);
                }
                temp = temp.parent(version);
            }
        }
    }

    unsafe fn set_modification(&mut self, mod_data: ModData<K, V>, version: usize) {
        if (*self.0).mods.len() < MAX_MODS {
            (*self.0).mods.push(Mod {
                data: mod_data,
                version,
            });
            return;
        }

        // Create a new node with all mods and the new change right here
        let new_gojo_node = (*self.0).clone_with_latest_mods();
        let new_node_ptr = NodePtr(Box::into_raw(Box::new(new_gojo_node)));
        match mod_data {
            ModData::Parent(p) => (*new_node_ptr.0).parent = p,
            ModData::Left(l) => (*new_node_ptr.0).left = l,
            ModData::Right(r) => (*new_node_ptr.0).right = r,
            ModData::Col(c) => (*new_node_ptr.0).color = c,
        }

        // Update left back pontairos
        let mut bk_ptr_left = (*new_node_ptr.0).bk_ptr_left;
        if !bk_ptr_left.is_null() {
            bk_ptr_left.set_parent(new_node_ptr, version);
        }

        // Update left back pontairos
        let mut bk_ptr_right = (*new_node_ptr.0).bk_ptr_right;
        if !bk_ptr_right.is_null() {
            bk_ptr_right.set_parent(new_node_ptr, version);
        }

        // Update parent back pontairos
        let mut bk_ptr_parent = (*new_node_ptr.0).bk_ptr_parent;
        if !bk_ptr_parent.is_null() {
            if new_node_ptr.is_left_child(version) {
                bk_ptr_parent.set_left(new_node_ptr, version);
            } else {
                bk_ptr_parent.set_right(new_node_ptr, version);
            }
        }
    }

    fn set_parent(&mut self, parent: NodePtr<K, V>, version: usize) {
        if self.is_null() {
            return;
        }
        unsafe {
            let new_mod = ModData::Parent(parent);
            self.set_modification(new_mod, version);
        }
    }

    fn set_left(&mut self, left: NodePtr<K, V>, version: usize) {
        if self.is_null() {
            return;
        }
        unsafe {
            let new_mod = ModData::Left(left);
            self.set_modification(new_mod, version);
        }
    }

    fn set_right(&mut self, right: NodePtr<K, V>, version: usize) {
        if self.is_null() {
            return;
        }
        unsafe {
            let new_mod = ModData::Right(right);
            self.set_modification(new_mod, version);
        }
    }

    fn parent(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            return NodePtr::null();
        }
        unsafe {
            let mut value = (*self.0).parent;
            for m in (*self.0).mods.iter() {
                if m.version > version {
                    break;
                }
                if let ModData::Parent(d) = m.data {
                    value = d;
                }
            }
            value
        }
    }

    fn left(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            return NodePtr::null();
        }

        unsafe {
            let mut value = (*self.0).left;
            for m in (*self.0).mods.iter() {
                if m.version > version {
                    break;
                }
                if let ModData::Left(d) = m.data {
                    value = d;
                }
            }
            value
        }
    }

    fn right(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            return NodePtr::null();
        }
        unsafe {
            let mut value = (*self.0).right;
            for m in (*self.0).mods.iter() {
                if m.version > version {
                    break;
                }
                if let ModData::Right(d) = m.data {
                    value = d;
                }
            }
            value
        }
    }

    fn null() -> NodePtr<K, V> {
        NodePtr(ptr::null_mut())
    }

    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl<K: Ord + Clone, V: Clone> NodePtr<K, V> {
    unsafe fn deep_clone(&self, version: usize) -> NodePtr<K, V> {
        let mut node = NodePtr::new((*self.0).key.clone(), (*self.0).value.clone());
        if !self.left(version).is_null() {
            node.set_left(self.left(version).deep_clone(version), version);
            node.left(version).set_parent(node, version);
        }
        if !self.right(version).is_null() {
            node.set_right(self.right(version).deep_clone(version), version);
            node.right(version).set_parent(node, version);
        }
        node
    }
}

pub struct Gojo<K: Ord + Clone, V: Clone> {
    root: NodePtr<K, V>,
    len: usize,
    curr_version: usize,
    roots: Vec<NodePtr<K, V>>,
}

unsafe impl<K: Ord + Clone, V: Clone> Send for Gojo<K, V> {}

unsafe impl<K: Ord + Clone, V: Clone> Sync for Gojo<K, V> {}

// Drop all owned pointers if the tree is dropped
impl<K: Ord + Clone, V: Clone> Drop for Gojo<K, V> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// If key and value are both impl Clone, we can call clone to get a copy.
impl<K: Ord + Clone, V: Clone> Clone for Gojo<K, V> {
    fn clone(&self) -> Gojo<K, V> {
        unsafe {
            let mut new = Gojo::new();
            new.root = self.root.deep_clone(self.curr_version);
            new.len = self.len;
            new
        }
    }
}

pub struct Iter<'a, K: Ord + Clone + 'a, V: Clone + 'a> {
    head: NodePtr<K, V>,
    tail: NodePtr<K, V>,
    len: usize,
    version: usize,
    _marker: marker::PhantomData<&'a ()>,
}

impl<'a, K: Ord + Clone + 'a, V: Clone + 'a> Clone for Iter<'a, K, V> {
    fn clone(&self) -> Iter<'a, K, V> {
        Iter {
            head: self.head,
            tail: self.tail,
            len: self.len,
            version: self.version,
            _marker: self._marker,
        }
    }
}

impl<'a, K: Ord + Clone + 'a, V: Clone + 'a> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.len == 0 {
            return None;
        }

        if self.head.is_null() {
            return None;
        }

        let (k, v) = unsafe { (&(*self.head.0).key, &(*self.head.0).value) };
        self.head = self.head.next(self.version);
        self.len -= 1;
        Some((k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K: Ord + Clone + 'a, V: Clone + 'a> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        // println!("len = {:?}", self.len);
        if self.len == 0 {
            return None;
        }

        let (k, v) = unsafe { (&(*self.tail.0).key, &(*self.tail.0).value) };
        self.tail = self.tail.prev(self.version);
        self.len -= 1;
        Some((k, v))
    }
}

impl<K: Ord + Clone, V: Clone> Debug for Gojo<K, V>
where
    K: Ord + Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// This is a method to help us to get inner struct.
impl<K: Ord + Clone + Debug, V: Debug + Clone> Gojo<K, V> {
    fn tree_print(&self, node: NodePtr<K, V>, direction: i32, version: usize) {
        if node.is_null() {
            return;
        }
        if direction == 0 {
            unsafe {
                println!("'{:?}' is root node", (*node.0));
            }
        } else {
            let direct = if direction == -1 { "left" } else { "right" };
            unsafe {
                println!(
                    "{:?} is {:?}'s {:?} child ",
                    (*node.0),
                    *node.parent(version).0,
                    direct
                );
            }
        }
        self.tree_print(node.left(version), -1, version);
        self.tree_print(node.right(version), 1, version);
    }

    pub fn print_tree(&self, version: usize) {
        if self.root.is_null() {
            println!("This is a empty tree");
            return;
        }
        println!("This tree size = {:?}, begin:-------------", self.len());
        self.tree_print(self.root, 0, version);
        println!("end--------------------------");
    }
}

impl<K, V> PartialEq for Gojo<K, V>
where
    K: Eq + Ord + Clone,
    V: PartialEq + Clone,
{
    fn eq(&self, other: &Gojo<K, V>) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter().all(|(key, value)| {
            other
                .get(key, self.curr_version)
                .map_or(false, |v| *value == *v)
        })
    }
}

impl<K, V> Eq for Gojo<K, V>
where
    K: Eq + Ord + Clone,
    V: Eq + Clone,
{
}

impl<'a, K, V> Index<&'a K> for Gojo<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    type Output = V;

    fn index(&self, index: &K) -> &V {
        self.get(index, self.curr_version)
            .expect("no entry found for key")
    }
}

impl<K: Ord + Clone, V: Clone> Gojo<K, V> {
    /// Creates an empty `RBTree`.
    pub fn new() -> Gojo<K, V> {
        let roots = Vec::from([NodePtr::<K, V>::null()]);
        Gojo {
            root: NodePtr::null(),
            len: 0,
            curr_version: 0,
            roots,
        }
    }

    /// Returns the len of `RBTree`.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return the current version
    pub fn version(&self) -> usize {
        self.curr_version
    }

    /// Returns `true` if the `RBTree` is empty.
    pub fn is_empty(&self) -> bool {
        self.root.is_null()
    }

    unsafe fn left_rotate(&mut self, mut node: NodePtr<K, V>) {
        let mut temp = node.right(self.curr_version);
        node.set_right(temp.left(self.curr_version), self.curr_version);

        if !temp.left(self.curr_version).is_null() {
            temp.left(self.curr_version)
                .set_parent(node, self.curr_version);
        }

        temp.set_parent(node.parent(self.curr_version), self.curr_version);
        if node == self.root {
            self.root = temp;
        } else if node == node.parent(self.curr_version).left(self.curr_version) {
            node.parent(self.curr_version)
                .set_left(temp, self.curr_version);
        } else {
            node.parent(self.curr_version)
                .set_right(temp, self.curr_version);
        }

        temp.set_left(node, self.curr_version);
        node.set_parent(temp, self.curr_version);
    }

    unsafe fn right_rotate(&mut self, mut node: NodePtr<K, V>) {
        let mut temp = node.left(self.curr_version);
        node.set_left(temp.right(self.curr_version), self.curr_version);

        if !temp.right(self.curr_version).is_null() {
            temp.right(self.curr_version)
                .set_parent(node, self.curr_version);
        }

        temp.set_parent(node.parent(self.curr_version), self.curr_version);
        if node == self.root {
            self.root = temp;
        } else if node == node.parent(self.curr_version).right(self.curr_version) {
            node.parent(self.curr_version)
                .set_right(temp, self.curr_version);
        } else {
            node.parent(self.curr_version)
                .set_left(temp, self.curr_version);
        }

        temp.set_right(node, self.curr_version);
        node.set_parent(temp, self.curr_version);
    }

    pub fn replace_or_insert(&mut self, k: K, mut v: V) -> Option<V> {
        let node = self.find_node(&k, self.curr_version);
        if node.is_null() {
            self.insert(k, v);
            return None;
        }

        unsafe {
            mem::swap(&mut v, &mut (*node.0).value);
        }

        Some(v)
    }

    unsafe fn insert_fixup(&mut self, mut node: NodePtr<K, V>) {
        let mut parent;
        let mut gparent;

        while node
            .parent(self.curr_version)
            .is_red_color(self.curr_version)
        {
            parent = node.parent(self.curr_version);
            gparent = parent.parent(self.curr_version);
            if parent == gparent.left(self.curr_version) {
                let mut uncle = gparent.right(self.curr_version);
                if !uncle.is_null() && uncle.is_red_color(self.curr_version) {
                    uncle.set_black_color();
                    parent.set_black_color();
                    gparent.set_red_color();
                    node = gparent;
                    continue;
                }

                if parent.right(self.curr_version) == node {
                    self.left_rotate(parent);
                    std::mem::swap(&mut parent, &mut node);
                }

                parent.set_black_color();
                gparent.set_red_color();
                self.right_rotate(gparent);
            } else {
                let mut uncle = gparent.left(self.curr_version);
                if !uncle.is_null() && uncle.is_red_color(self.curr_version) {
                    uncle.set_black_color();
                    parent.set_black_color();
                    gparent.set_red_color();
                    node = gparent;
                    continue;
                }

                if parent.left(self.curr_version) == node {
                    self.right_rotate(parent);
                    std::mem::swap(&mut parent, &mut node);
                }

                parent.set_black_color();
                gparent.set_red_color();
                self.left_rotate(gparent);
            }
        }
        self.root.set_black_color();
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.len += 1;
        let mut node = NodePtr::new(k, v);
        let mut y = NodePtr::null();
        let mut x = self.roots[self.curr_version];
        self.curr_version += 1;

        while !x.is_null() {
            y = x;
            match node.cmp(&x) {
                Ordering::Less => {
                    x = x.left(self.curr_version);
                }
                _ => {
                    x = x.right(self.curr_version);
                }
            };
        }
        node.set_parent(y, self.curr_version);

        if y.is_null() {
            self.root = node;
        } else {
            match node.cmp(&y) {
                Ordering::Less => {
                    y.set_left(node, self.curr_version);
                }
                _ => {
                    y.set_right(node, self.curr_version);
                }
            };
        }

        node.set_red_color();
        unsafe {
            self.insert_fixup(node);
        }

        self.roots.push(self.root);
    }

    fn find_node(&self, k: &K, version: usize) -> NodePtr<K, V> {
        if version > self.curr_version {
            return NodePtr::null();
        }

        let root = self.roots[version];
        let mut temp = root;
        unsafe {
            loop {
                let next = match k.cmp(&(*temp.0).key) {
                    Ordering::Less => temp.left(version),
                    Ordering::Greater => temp.right(version),
                    Ordering::Equal => return temp,
                };
                if next.is_null() {
                    break;
                }
                temp = next;
            }
        }
        NodePtr::null()
    }

    fn first_child(&self) -> NodePtr<K, V> {
        if self.root.is_null() {
            NodePtr::null()
        } else {
            let mut temp = self.root;
            while !temp.left(self.curr_version).is_null() {
                temp = temp.left(self.curr_version);
            }
            temp
        }
    }

    fn last_child(&self) -> NodePtr<K, V> {
        if self.root.is_null() {
            NodePtr::null()
        } else {
            let mut temp = self.root;
            while !temp.right(self.curr_version).is_null() {
                temp = temp.right(self.curr_version);
            }
            temp
        }
    }

    pub fn get_first(&self) -> Option<(&K, &V)> {
        let first = self.first_child();
        if first.is_null() {
            return None;
        }
        unsafe { Some((&(*first.0).key, &(*first.0).value)) }
    }

    pub fn get_last(&self) -> Option<(&K, &V)> {
        let last = self.last_child();
        if last.is_null() {
            return None;
        }
        unsafe { Some((&(*last.0).key, &(*last.0).value)) }
    }

    pub fn pop_first(&mut self) -> Option<(K, V)> {
        let first = self.first_child();
        if first.is_null() {
            return None;
        }
        unsafe { Some(self.delete(first)) }
    }

    pub fn pop_last(&mut self) -> Option<(K, V)> {
        let last = self.last_child();
        if last.is_null() {
            return None;
        }
        unsafe { Some(self.delete(last)) }
    }

    pub fn get_first_mut(&mut self) -> Option<(&K, &mut V)> {
        let first = self.first_child();
        if first.is_null() {
            return None;
        }
        unsafe { Some((&(*first.0).key, &mut (*first.0).value)) }
    }

    pub fn get_last_mut(&mut self) -> Option<(&K, &mut V)> {
        let last = self.last_child();
        if last.is_null() {
            return None;
        }
        unsafe { Some((&(*last.0).key, &mut (*last.0).value)) }
    }

    pub fn get(&self, k: &K, version: usize) -> Option<&V> {
        let node = self.find_node(k, version);
        if node.is_null() {
            return None;
        }

        unsafe { Some(&(*node.0).value) }
    }

    pub fn get_mut(&mut self, k: &K, version: usize) -> Option<&mut V> {
        let node = self.find_node(k, version);
        if node.is_null() {
            return None;
        }

        unsafe { Some(&mut (*node.0).value) }
    }

    pub fn contains_key(&self, k: &K, version: usize) -> bool {
        let node = self.find_node(k, version);
        if node.is_null() {
            return false;
        }
        true
    }

    fn clear_recurse(&mut self, current: NodePtr<K, V>) {
        if !current.is_null() {
            unsafe {
                self.clear_recurse(current.left(self.curr_version));
                self.clear_recurse(current.right(self.curr_version));
                let _ = Box::from_raw(current.0);
            }
        }
    }

    pub fn clear(&mut self) {
        let root = self.root;
        self.root = NodePtr::null();
        self.clear_recurse(root);
        self.len = 0;
    }

    fn fast_clear(&mut self) {
        self.root = NodePtr::null();
        self.len = 0;
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let node = self.find_node(k, self.curr_version);
        if node.is_null() {
            return None;
        }
        unsafe { Some(self.delete(node).1) }
    }

    unsafe fn delete_fixup(&mut self, mut node: NodePtr<K, V>, mut parent: NodePtr<K, V>) {
        let mut other;
        while node != self.root && node.is_black_color(self.curr_version) {
            if parent.left(self.curr_version) == node {
                other = parent.right(self.curr_version);
                if other.is_red_color(self.curr_version) {
                    other.set_black_color();
                    parent.set_red_color();
                    self.left_rotate(parent);
                    other = parent.right(self.curr_version);
                }

                if other
                    .left(self.curr_version)
                    .is_black_color(self.curr_version)
                    && other
                        .right(self.curr_version)
                        .is_black_color(self.curr_version)
                {
                    other.set_red_color();
                    node = parent;
                    parent = node.parent(self.curr_version);
                } else {
                    if other
                        .right(self.curr_version)
                        .is_black_color(self.curr_version)
                    {
                        other.left(self.curr_version).set_black_color();
                        other.set_red_color();
                        self.right_rotate(other);
                        other = parent.right(self.curr_version);
                    }
                    other.set_color(parent.get_color(self.curr_version));
                    parent.set_black_color();
                    other.right(self.curr_version).set_black_color();
                    self.left_rotate(parent);
                    node = self.root;
                    break;
                }
            } else {
                other = parent.left(self.curr_version);
                if other.is_red_color(self.curr_version) {
                    other.set_black_color();
                    parent.set_red_color();
                    self.right_rotate(parent);
                    other = parent.left(self.curr_version);
                }

                if other
                    .left(self.curr_version)
                    .is_black_color(self.curr_version)
                    && other
                        .right(self.curr_version)
                        .is_black_color(self.curr_version)
                {
                    other.set_red_color();
                    node = parent;
                    parent = node.parent(self.curr_version);
                } else {
                    if other
                        .left(self.curr_version)
                        .is_black_color(self.curr_version)
                    {
                        other.right(self.curr_version).set_black_color();
                        other.set_red_color();
                        self.left_rotate(other);
                        other = parent.left(self.curr_version);
                    }
                    other.set_color(parent.get_color(self.curr_version));
                    parent.set_black_color();
                    other.left(self.curr_version).set_black_color();
                    self.right_rotate(parent);
                    node = self.root;
                    break;
                }
            }
        }

        node.set_black_color();
    }

    unsafe fn delete(&mut self, node: NodePtr<K, V>) -> (K, V) {
        let mut child;
        let mut parent;
        let color;

        self.len -= 1;
        if !node.left(self.curr_version).is_null() && !node.right(self.curr_version).is_null() {
            let mut replace = node.right(self.curr_version).min_node(self.curr_version);
            if node == self.root {
                self.root = replace;
            } else if node.parent(self.curr_version).left(self.curr_version) == node {
                node.parent(self.curr_version)
                    .set_left(replace, self.curr_version);
            } else {
                node.parent(self.curr_version)
                    .set_right(replace, self.curr_version);
            }

            child = replace.right(self.curr_version);
            parent = replace.parent(self.curr_version);
            color = replace.get_color(self.curr_version);
            if parent == node {
                parent = replace;
            } else {
                if !child.is_null() {
                    child.set_parent(parent, self.curr_version);
                }
                parent.set_left(child, self.curr_version);
                replace.set_right(node.right(self.curr_version), self.curr_version);
                node.right(self.curr_version)
                    .set_parent(replace, self.curr_version);
            }

            replace.set_parent(node.parent(self.curr_version), self.curr_version);
            replace.set_color(node.get_color(self.curr_version));
            replace.set_left(node.left(self.curr_version), self.curr_version);
            node.left(self.curr_version)
                .set_parent(replace, self.curr_version);

            if color == Color::Black {
                self.delete_fixup(child, parent);
            }

            let obj = Box::from_raw(node.0);
            return obj.pair();
        }

        if !node.left(self.curr_version).is_null() {
            child = node.left(self.curr_version);
        } else {
            child = node.right(self.curr_version);
        }

        parent = node.parent(self.curr_version);
        color = node.get_color(self.curr_version);
        if !child.is_null() {
            child.set_parent(parent, self.curr_version);
        }

        if self.root == node {
            self.root = child
        } else if parent.left(self.curr_version) == node {
            parent.set_left(child, self.curr_version);
        } else {
            parent.set_right(child, self.curr_version);
        }

        if color == Color::Black {
            self.delete_fixup(child, parent);
        }

        let obj = Box::from_raw(node.0);
        obj.pair()
    }

    /// Return the key and value iter
    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            head: self.first_child(),
            tail: self.last_child(),
            len: self.len,
            version: self.curr_version,
            _marker: marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Gojo;

    #[test]
    fn test_insert() {
        let mut m = Gojo::new();
        m.insert(1, 2);
        m.insert(2, 4);
        m.insert(2, 6);
        assert_eq!(m.len(), 3);

        let version = 3;
        assert_eq!(m.version(), version);

        assert_eq!(*m.get(&1, version).unwrap(), 2);
        assert_eq!(*m.get(&2, version).unwrap(), 4);
        assert_eq!(*m.get(&2, version).unwrap(), 4);
    }

    #[test]
    fn test_cant_find_element_without_correct_version() {
        let mut m = Gojo::new();
        for key in 1..10 {
            m.insert(key, key << 2);
        }

        let version = 6;
        let key = 7;
        assert!(m.get(&key, version).is_none());
    }

    // #[test]
    // fn test_replace() {
    //     let mut m = Gojo::new();
    //     assert_eq!(m.len(), 0);
    //     m.insert(2, 4);
    //     assert_eq!(m.len(), 1);
    //     assert_eq!(m.replace_or_insert(2, 6).unwrap(), 4);
    //     assert_eq!(m.len(), 1);
    //     assert_eq!(*m.get(&2).unwrap(), 6);
    // }
    //
    // #[test]
    // fn test_clone() {
    //     let mut m = Gojo::new();
    //     assert_eq!(m.len(), 0);
    //     m.insert(1, 2);
    //     assert_eq!(m.len(), 1);
    //     m.insert(2, 4);
    //     assert_eq!(m.len(), 2);
    //     let m2 = m.clone();
    //     m.clear();
    //     assert_eq!(*m2.get(&1).unwrap(), 2);
    //     assert_eq!(*m2.get(&2).unwrap(), 4);
    //     assert_eq!(m2.len(), 2);
    // }
    //
    // #[test]
    // fn test_empty_remove() {
    //     let mut m: Gojo<isize, bool> = Gojo::new();
    //     assert_eq!(m.remove(&0), None);
    // }
    //
    // #[test]
    // fn test_lots_of_insertions() {
    //     let mut m = Gojo::new();
    //
    //     // Try this a few times to make sure we never screw up the hashmap's
    //     // internal state.
    //     for _ in 0..10 {
    //         assert!(m.is_empty());
    //
    //         for i in 1..101 {
    //             m.insert(i, i);
    //
    //             for j in 1..i + 1 {
    //                 let r = m.get(&j);
    //                 assert_eq!(r, Some(&j));
    //             }
    //
    //             for j in i + 1..101 {
    //                 let r = m.get(&j);
    //                 assert_eq!(r, None);
    //             }
    //         }
    //
    //         for i in 101..201 {
    //             assert!(!m.contains_key(&i));
    //         }
    //
    //         // remove forwards
    //         for i in 1..101 {
    //             assert!(m.remove(&i).is_some());
    //
    //             for j in 1..i + 1 {
    //                 assert!(!m.contains_key(&j));
    //             }
    //
    //             for j in i + 1..101 {
    //                 assert!(m.contains_key(&j));
    //             }
    //         }
    //
    //         for i in 1..101 {
    //             assert!(!m.contains_key(&i));
    //         }
    //
    //         for i in 1..101 {
    //             m.insert(i, i);
    //         }
    //
    //         // remove backwards
    //         for i in (1..101).rev() {
    //             assert!(m.remove(&i).is_some());
    //
    //             for j in i..101 {
    //                 assert!(!m.contains_key(&j));
    //             }
    //
    //             for j in 1..i {
    //                 let kkk = m.contains_key(&j);
    //
    //                 assert!(m.contains_key(&j));
    //             }
    //         }
    //     }
    // }
    //
    // #[test]
    // fn test_find_mut() {
    //     let mut m = Gojo::new();
    //     m.insert(1, 12);
    //     m.insert(2, 8);
    //     m.insert(5, 14);
    //     let new = 100;
    //     match m.get_mut(&5) {
    //         None => panic!(),
    //         Some(x) => *x = new,
    //     }
    //     assert_eq!(m.get(&5), Some(&new));
    // }
    //
    // #[test]
    // fn test_remove() {
    //     let mut m = Gojo::new();
    //     m.insert(1, 2);
    //     assert_eq!(*m.get(&1).unwrap(), 2);
    //     m.insert(5, 3);
    //     assert_eq!(*m.get(&5).unwrap(), 3);
    //     m.insert(9, 4);
    //     assert_eq!(*m.get(&1).unwrap(), 2);
    //     assert_eq!(*m.get(&5).unwrap(), 3);
    //     assert_eq!(*m.get(&9).unwrap(), 4);
    //     assert_eq!(m.remove(&1).unwrap(), 2);
    //     assert_eq!(m.remove(&5).unwrap(), 3);
    //     assert_eq!(m.remove(&9).unwrap(), 4);
    //     assert_eq!(m.len(), 0);
    // }
    //
    // #[test]
    // fn test_is_empty() {
    //     let mut m = Gojo::new();
    //     m.insert(1, 2);
    //     assert!(!m.is_empty());
    //     assert!(m.remove(&1).is_some());
    //     assert!(m.is_empty());
    // }
    //
    // #[test]
    // fn test_pop() {
    //     let mut m = Gojo::new();
    //     m.insert(2, 4);
    //     m.insert(1, 2);
    //     m.insert(3, 6);
    //     assert_eq!(m.len(), 3);
    //     assert_eq!(m.pop_first(), Some((1, 2)));
    //     assert_eq!(m.len(), 2);
    //     assert_eq!(m.pop_last(), Some((3, 6)));
    //     assert_eq!(m.len(), 1);
    //     assert_eq!(m.get_first(), Some((&2, &4)));
    //     assert_eq!(m.get_last(), Some((&2, &4)));
    // }
    //
    // #[test]
    // fn test_eq() {
    //     let mut m1 = Gojo::new();
    //     m1.insert(1, 2);
    //     m1.insert(2, 3);
    //     m1.insert(3, 4);
    //
    //     let mut m2 = Gojo::new();
    //     m2.insert(1, 2);
    //     m2.insert(2, 3);
    //
    //     assert!(m1 != m2);
    //
    //     m2.insert(3, 4);
    //
    //     assert_eq!(m1, m2);
    // }
    //
    // #[test]
    // fn test_show() {
    //     let mut map = Gojo::new();
    //     let empty: Gojo<i32, i32> = Gojo::new();
    //
    //     map.insert(1, 2);
    //     map.insert(3, 4);
    //
    //     let map_str = format!("{:?}", map);
    //
    //     assert!(map_str == "{1: 2, 3: 4}" || map_str == "{3: 4, 1: 2}");
    //     assert_eq!(format!("{:?}", empty), "{}");
    // }
    //
    // #[test]
    // fn test_index() {
    //     let mut map = Gojo::new();
    //
    //     map.insert(1, 2);
    //     map.insert(2, 1);
    //     map.insert(3, 4);
    //
    //     assert_eq!(map[&2], 1);
    // }
    //
    // #[test]
    // #[should_panic]
    // fn test_index_nonexistent() {
    //     let mut map = Gojo::new();
    //
    //     map.insert(1, 2);
    //     map.insert(2, 1);
    //     map.insert(3, 4);
    //
    //     assert!(map.len() > 4);
    // }
}
