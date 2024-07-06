use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::ptr;

const MAX_MODS: usize = 6;
const MAX_OPS: usize = 20;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
enum Color {
    #[default]
    Red,
    Black,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ModData<K: Ord + Clone + Default, V: Clone + Default> {
    Parent(NodePtr<K, V>),
    Left(NodePtr<K, V>),
    Right(NodePtr<K, V>),
    Col(Color),
}

struct Mod<K: Ord + Clone + Default, V: Clone + Default> {
    data: ModData<K, V>,
    version: usize,
}

impl<K: Ord + Clone + Default, V: Clone + Default> Mod<K, V> {
    fn new(data: ModData<K, V>, version: usize) -> Self {
        Self { data, version }
    }
}

struct GojoNode<K: Ord + Clone + Default, V: Clone + Default> {
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
    next_copy: NodePtr<K, V>,
    version: usize,
}

impl<K: Ord + Clone + Default, V: Clone + Default> GojoNode<K, V> {
    fn clone_with_latest_mods(&self) -> Self {
        let key = self.key.clone();
        let value = self.value.clone();
        let mods = Vec::with_capacity(MAX_MODS);
        let next_copy = NodePtr::null();

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

        let version = match self.mods.last() {
            Some(m) => m.version,
            None => self.version,
        };
        let bk_ptr_left = left;
        let bk_ptr_right = right;
        let bk_ptr_parent = parent;
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
            next_copy,
            version,
        }
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> Default for GojoNode<K, V> {
    fn default() -> Self {
        Self {
            color: Default::default(),
            left: NodePtr::null(),
            right: NodePtr::null(),
            parent: NodePtr::null(),
            bk_ptr_left: NodePtr::null(),
            bk_ptr_right: NodePtr::null(),
            bk_ptr_parent: NodePtr::null(),
            key: Default::default(),
            value: Default::default(),
            mods: Default::default(),
            next_copy: NodePtr::null(),
            version: 0,
        }
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> GojoNode<K, V> {
    fn pair(self) -> (K, V) {
        (self.key, self.value)
    }
}

impl<K, V> Debug for GojoNode<K, V>
where
    K: Ord + Debug + Clone + Default,
    V: Debug + Clone + Default,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "k:{:?} v:{:?} c:{:?}", self.key, self.value, self.color)
    }
}

#[derive(Debug)]
struct NodePtr<K: Ord + Clone + Default, V: Clone + Default> {
    pointer: *mut GojoNode<K, V>,
    null: bool,
}

impl<K: Ord + Clone + Default, V: Clone + Default> Clone for NodePtr<K, V> {
    fn clone(&self) -> NodePtr<K, V> {
        *self
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> Copy for NodePtr<K, V> {}

impl<K: Ord + Clone + Default, V: Clone + Default> Ord for NodePtr<K, V> {
    fn cmp(&self, other: &NodePtr<K, V>) -> Ordering {
        unsafe { (*self.pointer).key.cmp(&(*other.pointer).key) }
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> PartialOrd for NodePtr<K, V> {
    fn partial_cmp(&self, other: &NodePtr<K, V>) -> Option<Ordering> {
        unsafe { Some((*self.pointer).key.cmp(&(*other.pointer).key)) }
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

impl<K: Ord + Clone + Default, V: Clone + Default> PartialEq for NodePtr<K, V> {
    fn eq(&self, other: &NodePtr<K, V>) -> bool {
        unsafe { (*self.pointer).key == (*other.pointer).key }
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> Eq for NodePtr<K, V> {}

impl<K: Ord + Clone + Default, V: Clone + Default> From<GojoNode<K, V>> for NodePtr<K, V> {
    fn from(value: GojoNode<K, V>) -> Self {
        let ptr = Box::into_raw(Box::new(value));

        NodePtr {
            pointer: ptr,
            null: false,
        }
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> NodePtr<K, V> {
    fn new(k: K, v: V) -> NodePtr<K, V> {
        let node = GojoNode {
            key: k,
            value: v,
            ..Default::default()
        };
        NodePtr {
            pointer: Box::into_raw(Box::new(node)),
            null: false,
        }
    }

    fn set_color(&mut self, color: Color, version: usize) -> Option<NodePtr<K, V>> {
        debug_assert!(!self.is_null(), "Should not update a color on a null node");

        let curr_color = self.get_color(version);
        if color == curr_color {
            return None;
        }

        unsafe {
            let new_mod = ModData::Col(color);
            self.set_modification(new_mod, version)
        }
    }

    fn set_red_color(&mut self, version: usize) -> Option<NodePtr<K, V>> {
        self.set_color(Color::Red, version)
    }

    fn set_black_color(&mut self, version: usize) -> Option<NodePtr<K, V>> {
        self.set_color(Color::Black, version)
    }

    unsafe fn get_next_copy(&self) -> NodePtr<K, V> {
        (*self.pointer).next_copy
    }

    fn get_latest_copy_for_version(&self, version: usize) -> NodePtr<K, V> {
        let mut caba = *self;
        unsafe {
            while !caba.get_next_copy().is_null() && caba.get_next_copy().version() <= version {
                caba = caba.get_next_copy();
            }
        }
        caba
    }

    fn get_color(&self, version: usize) -> Color {
        if self.is_null() {
            return Color::Black;
        }
        unsafe {
            let ptr = self.get_latest_copy_for_version(version);
            let mut value = (*ptr.pointer).color;
            for m in (*ptr.pointer).mods.iter() {
                if m.version > version {
                    break;
                }
                if let ModData::Col(d) = m.data {
                    value = d;
                }
            }
            value
        }
    }

    fn version(&self) -> usize {
        unsafe { (*self.pointer).version }
    }

    fn is_red_color(&self, version: usize) -> bool {
        if self.is_null() {
            return false;
        }
        let color = self.get_color(version);
        color == Color::Red
    }

    fn is_black_color(&self, version: usize) -> bool {
        if self.is_null() {
            return true;
        }
        let color = self.get_color(version);
        color == Color::Black
    }

    fn is_left_child(&self, version: usize) -> bool {
        unsafe { (*self.parent(version).left(version).pointer).key == (*self.pointer).key }
    }

    fn is_right_child(&self, version: usize) -> bool {
        unsafe { (*self.parent(version).right(version).pointer).key == (*self.pointer).key }
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

    unsafe fn set_modification(
        &mut self,
        mod_data: ModData<K, V>,
        version: usize,
    ) -> Option<NodePtr<K, V>> {
        if (*self.pointer).mods.len() < MAX_MODS {
            match mod_data {
                ModData::Parent(p) => (*self.pointer).bk_ptr_parent = p,
                ModData::Left(l) => (*self.pointer).bk_ptr_left = l,
                ModData::Right(r) => (*self.pointer).bk_ptr_right = r,
                ModData::Col(_) => (),
            }
            (*self.pointer).mods.push(Mod {
                data: mod_data,
                version,
            });
            return None;
        }

        // Create a new node with all mods and the new change right here
        let new_gojo_node = (*self.pointer).clone_with_latest_mods();
        let new_node_ptr = NodePtr {
            pointer: Box::into_raw(Box::new(new_gojo_node)),
            null: false,
        };
        match mod_data {
            ModData::Parent(p) => {
                (*new_node_ptr.pointer).parent = p;
                (*new_node_ptr.pointer).bk_ptr_parent = p;
            }
            ModData::Left(l) => {
                (*new_node_ptr.pointer).left = l;
                (*new_node_ptr.pointer).bk_ptr_left = l
            }
            ModData::Right(r) => {
                (*new_node_ptr.pointer).right = r;
                (*new_node_ptr.pointer).bk_ptr_right = r
            }
            ModData::Col(c) => (*new_node_ptr.pointer).color = c,
        }

        // Update left back pontairos
        let mut bk_ptr_left = (*new_node_ptr.pointer).bk_ptr_left;
        if !bk_ptr_left.is_null() {
            bk_ptr_left.set_parent(new_node_ptr, version);
        }

        // Update left back pontairos
        let mut bk_ptr_right = (*new_node_ptr.pointer).bk_ptr_right;
        if !bk_ptr_right.is_null() {
            bk_ptr_right.set_parent(new_node_ptr, version);
        }

        let mut bk_ptr_parent = (*new_node_ptr.pointer).bk_ptr_parent;

        // We got a new root boys
        if bk_ptr_parent.is_null() {
            return Some(new_node_ptr);
        }

        // Update parent back pontairos that can have a new root
        let possible_new_root: Option<NodePtr<K, V>>;
        if new_node_ptr.is_left_child(version) {
            possible_new_root = bk_ptr_parent.set_left(new_node_ptr, version);
        } else {
            possible_new_root = bk_ptr_parent.set_right(new_node_ptr, version);
        }

        possible_new_root
    }

    fn set_parent(&mut self, parent: NodePtr<K, V>, version: usize) -> Option<NodePtr<K, V>> {
        debug_assert!(!self.is_null(), "Trying to change parent for null node");
        unsafe {
            let new_mod = ModData::Parent(parent);
            self.set_modification(new_mod, version)
        }
    }

    fn set_left(&mut self, left: NodePtr<K, V>, version: usize) -> Option<NodePtr<K, V>> {
        debug_assert!(!self.is_null(), "Trying to change left for null node");

        unsafe {
            let new_mod = ModData::Left(left);
            self.set_modification(new_mod, version)
        }
    }

    fn set_right(&mut self, right: NodePtr<K, V>, version: usize) -> Option<NodePtr<K, V>> {
        debug_assert!(!self.is_null(), "Trying to change right for null node");

        unsafe {
            let new_mod = ModData::Right(right);
            self.set_modification(new_mod, version)
        }
    }

    fn parent(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            return NodePtr::null();
        }
        unsafe {
            let mut value = (*self.pointer).parent;
            for m in (*self.pointer).mods.iter() {
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
            let mut value = (*self.pointer).left;
            for m in (*self.pointer).mods.iter() {
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
            let mut value = (*self.pointer).right;
            for m in (*self.pointer).mods.iter() {
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
        NodePtr {
            pointer: ptr::null_mut(),
            null: true,
        }
    }

    fn is_null(&self) -> bool {
        self.null == true
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> NodePtr<K, V> {
    unsafe fn deep_clone(&self, version: usize) -> NodePtr<K, V> {
        let mut node = NodePtr::new((*self.pointer).key.clone(), (*self.pointer).value.clone());
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

pub struct Gojo<K: Ord + Clone + Default, V: Clone + Default> {
    root: NodePtr<K, V>,
    len: usize,
    curr_version: usize,
    roots: Vec<NodePtr<K, V>>,
    nil: NodePtr<K, V>,
}

unsafe impl<K: Ord + Clone + Default, V: Clone + Default> Send for Gojo<K, V> {}

unsafe impl<K: Ord + Clone + Default, V: Clone + Default> Sync for Gojo<K, V> {}

// Drop all owned pointers if the tree is dropped
impl<K: Ord + Clone + Default, V: Clone + Default> Drop for Gojo<K, V> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// If key and value are both impl Clone, we can call clone to get a copy.
impl<K: Ord + Clone + Default, V: Clone + Default> Clone for Gojo<K, V> {
    fn clone(&self) -> Gojo<K, V> {
        unsafe {
            let mut new = Gojo::new();
            new.root = self.root.deep_clone(self.curr_version);
            new.len = self.len;
            new
        }
    }
}

impl<K: Ord + Clone + Default, V: Clone + Default> Gojo<K, V> {
    /// Creates an empty `RBTree`.
    pub fn new() -> Gojo<K, V> {
        let mut nil = NodePtr::new(K::default(), V::default());
        nil.null = true;
        unsafe {
            (*(nil.pointer)).color = Color::Black;
        }
        let roots = Vec::from([nil]);
        Gojo {
            root: NodePtr::null(),
            len: 0,
            curr_version: 0,
            roots,
            nil,
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

    fn set_possible_new_root(&mut self, possible_new_root: Option<NodePtr<K, V>>) {
        if possible_new_root.is_some() {
            self.root = possible_new_root.unwrap();
        }
    }

    pub fn predecessor(&self, node: NodePtr<K, V>) -> NodePtr<K, V> {
        todo!()
    }

    pub fn successor(&self, node: NodePtr<K, V>) -> NodePtr<K, V> {
        todo!()
    }

    unsafe fn left_rotate(&mut self, mut node: NodePtr<K, V>) {
        let mut temp = node.right(self.curr_version);
        self.set_possible_new_root(node.set_right(temp.left(self.curr_version), self.curr_version));

        if !temp.left(self.curr_version).is_null() {
            self.set_possible_new_root(
                temp.left(self.curr_version)
                    .set_parent(node, self.curr_version),
            );
        }

        self.set_possible_new_root(
            temp.set_parent(node.parent(self.curr_version), self.curr_version),
        );
        if node.parent(self.curr_version).is_null() {
            self.root = temp.clone();
        } else if node == temp.parent(self.curr_version).left(self.curr_version) {
            self.set_possible_new_root(
                node.parent(self.curr_version)
                    .set_left(temp, self.curr_version),
            );
        } else {
            self.set_possible_new_root(
                node.parent(self.curr_version)
                    .set_right(temp, self.curr_version),
            );
        }

        self.set_possible_new_root(temp.set_left(node, self.curr_version));
        self.set_possible_new_root(node.set_parent(temp, self.curr_version));
    }

    unsafe fn right_rotate(&mut self, mut node: NodePtr<K, V>) {
        let mut temp = node.left(self.curr_version);
        self.set_possible_new_root(node.set_left(temp.right(self.curr_version), self.curr_version));

        if !temp.right(self.curr_version).is_null() {
            self.set_possible_new_root(
                temp.right(self.curr_version)
                    .set_parent(node, self.curr_version),
            );
        }

        self.set_possible_new_root(
            temp.set_parent(node.parent(self.curr_version), self.curr_version),
        );
        if node.parent(self.curr_version).is_null() {
            self.root = temp.clone();
        } else if node == node.parent(self.curr_version).right(self.curr_version) {
            self.set_possible_new_root(
                node.parent(self.curr_version)
                    .set_right(temp, self.curr_version),
            );
        } else {
            self.set_possible_new_root(
                node.parent(self.curr_version)
                    .set_left(temp, self.curr_version),
            );
        }

        self.set_possible_new_root(temp.set_right(node, self.curr_version));
        self.set_possible_new_root(node.set_parent(temp, self.curr_version));
    }

    unsafe fn insert_fixup(&mut self, node: NodePtr<K, V>) {
        let mut dude = node;
        while dude != self.root
            && dude
                .parent(self.curr_version)
                .is_red_color(self.curr_version)
        {
            let mut parent = dude.parent(self.curr_version);
            let mut gparent = parent.parent(self.curr_version);
            if parent == gparent.left(self.curr_version) {
                let mut uncle = gparent.right(self.curr_version);

                // Case 1
                if uncle.is_red_color(self.curr_version) {
                    self.set_possible_new_root(parent.set_black_color(self.curr_version));
                    self.set_possible_new_root(uncle.set_black_color(self.curr_version));
                    self.set_possible_new_root(gparent.set_red_color(self.curr_version));
                    dude = gparent;
                    continue;
                }

                // Case 2
                if dude == parent.right(self.curr_version) {
                    dude = parent;
                    self.left_rotate(dude);
                }

                // Case 3
                self.set_possible_new_root(parent.set_black_color(self.curr_version));
                self.set_possible_new_root(gparent.set_red_color(self.curr_version));
                self.right_rotate(gparent);
            } else {
                let mut uncle = gparent.left(self.curr_version);

                // Case 4
                if uncle.is_red_color(self.curr_version) {
                    self.set_possible_new_root(uncle.set_black_color(self.curr_version));
                    self.set_possible_new_root(parent.set_black_color(self.curr_version));
                    self.set_possible_new_root(gparent.set_red_color(self.curr_version));
                    dude = gparent;
                    continue;
                }

                // Case 5
                if parent.left(self.curr_version) == dude {
                    dude = parent;
                    self.right_rotate(dude);
                }

                // Case 6
                self.set_possible_new_root(parent.set_black_color(self.curr_version));
                self.set_possible_new_root(gparent.set_red_color(self.curr_version));
                self.left_rotate(gparent);
            }
        }
        let possible_new_root = self.root.set_black_color(self.curr_version);
        self.set_possible_new_root(possible_new_root);
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.len += 1;
        let node = NodePtr::new(k, v);
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
        unsafe {
            (*node.pointer).parent = self.nil;
            (*node.pointer).left = self.nil;
            (*node.pointer).right = self.nil;
        }

        if y.is_null() {
            self.root = node;
        } else {
            unsafe {
                (*node.pointer).parent = y;
                (*node.pointer).bk_ptr_parent = y;
            }
            match node.cmp(&y) {
                Ordering::Less => {
                    self.set_possible_new_root(y.set_left(node, self.curr_version));
                }
                _ => {
                    self.set_possible_new_root(y.set_right(node, self.curr_version));
                }
            };
        }

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
                let next = match k.cmp(&(*temp.pointer).key) {
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

    pub fn get(&self, k: &K, version: usize) -> Option<&V> {
        let node = self.find_node(k, version);
        if node.is_null() {
            return None;
        }

        unsafe { Some(&(*node.pointer).value) }
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
                let _ = Box::from_raw(current.pointer);
            }
        }
    }

    pub fn clear(&mut self) {
        let root = self.root;
        self.root = NodePtr::null();
        self.clear_recurse(root);
        self.len = 0;
        self.roots = Vec::new();
    }

    fn fast_clear(&mut self) {
        self.root = NodePtr::null();
        self.len = 0;
        self.roots = Vec::new();
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let node = self.find_node(k, self.curr_version);
        if node.is_null() {
            return None;
        }
        self.len -= 1;
        unsafe { Some(self.delete(node).1) }
    }

    unsafe fn delete_fixup(&mut self, x: NodePtr<K, V>) {
        let mut caba = x;
        let version = self.curr_version;

        while !caba.is_null() && caba.is_black_color(version) {
            if caba.is_left_child(version) {
                let mut w = caba.parent(version).right(version);

                // Case 1
                if w.is_red_color(version) {
                    self.set_possible_new_root(w.set_black_color(version));
                    self.set_possible_new_root(caba.parent(version).set_red_color(version));
                    self.left_rotate(caba.parent(version));
                    w = caba.parent(version).right(version);
                }

                // Case 2
                if w.left(version).is_black_color(version)
                    == w.right(version).is_black_color(version)
                {
                    self.set_possible_new_root(w.set_red_color(version));
                    caba = caba.parent(version);
                }
                // Case 3
                else if w.right(version).is_black_color(version) {
                    self.set_possible_new_root(w.left(version).set_black_color(version));
                    self.set_possible_new_root(w.set_red_color(version));
                    self.right_rotate(caba.parent(version));
                    w = x.parent(version).right(version);
                }

                // Case 4
                self.set_possible_new_root(
                    w.set_color(caba.parent(version).get_color(version), version),
                );
                self.set_possible_new_root(caba.parent(version).set_black_color(version));
                self.set_possible_new_root(w.right(version).set_black_color(version));
                self.left_rotate(caba.parent(version));
                caba = self.root;
            } else {
                let mut w = caba.parent(version).left(version);

                // Case 1
                if w.is_red_color(version) {
                    self.set_possible_new_root(w.set_black_color(version));
                    self.set_possible_new_root(caba.parent(version).set_red_color(version));
                    self.right_rotate(caba.parent(version));
                    w = caba.parent(version).left(version);
                }

                // Case 2
                if w.right(version).is_black_color(version)
                    == w.left(version).is_black_color(version)
                {
                    self.set_possible_new_root(w.set_red_color(version));
                    caba = caba.parent(version);
                }
                // Case 3
                else if w.left(version).is_black_color(version) {
                    self.set_possible_new_root(w.right(version).set_black_color(version));
                    self.set_possible_new_root(w.set_red_color(version));
                    self.left_rotate(caba.parent(version));
                    w = x.parent(version).left(version);
                }

                // Case 4
                self.set_possible_new_root(
                    w.set_color(caba.parent(version).get_color(version), version),
                );
                self.set_possible_new_root(caba.parent(version).set_black_color(version));
                self.set_possible_new_root(w.left(version).set_black_color(version));
                self.right_rotate(caba.parent(version));
                caba = self.root;
            }

            self.set_possible_new_root(caba.set_black_color(version));
        }
    }

    unsafe fn transplant(&mut self, u: NodePtr<K, V>, mut v: NodePtr<K, V>) {
        let version = self.curr_version;

        if u.parent(version).is_null() {
            self.root = v;
        } else if u.is_left_child(version) {
            self.set_possible_new_root(u.parent(version).set_left(v, version));
        } else {
            self.set_possible_new_root(u.parent(version).set_right(v, version));
        }

        self.set_possible_new_root(v.set_parent(u.parent(version), version));
    }

    unsafe fn delete(&mut self, z: NodePtr<K, V>) -> (K, V) {
        let version = self.curr_version;
        let mut y = z;
        let mut original_y_color = y.get_color(version);
        let x;

        if z.left(version).is_null() {
            x = z.right(version);
            self.transplant(z, x);
        } else if z.right(version).is_null() {
            x = z.left(version);
            self.transplant(z, x);
        } else {
            y = self.successor(z);
            x = y.right(version);
            original_y_color = y.get_color(version);
            self.set_possible_new_root(y.set_left(z.left(version), version));
            self.set_possible_new_root(z.left(version).set_parent(y, version));
            self.set_possible_new_root(y.set_right(z.right(version), version));
            self.set_possible_new_root(z.right(version).set_parent(y, version));
            self.transplant(z, y);
            self.set_possible_new_root(y.set_color(z.get_color(version), version));
        }

        if original_y_color == Color::Black {
            self.delete_fixup(x);
        }

        let kv = Box::from_raw(y.pointer);
        kv.pair()
    }
}

#[cfg(test)]
mod tests {
    use crate::gojo::{Color, Mod, ModData, NodePtr};

    use super::{Gojo, GojoNode};

    #[test]
    fn test_get_color_without_mods() {
        // Arrange
        let no_mods_node = GojoNode {
            color: Color::Black,
            ..Default::default()
        };
        let version = 1;

        // Act
        let ptr = NodePtr::<i32, i32>::from(no_mods_node);

        // Assert
        assert_eq!(ptr.get_color(version), Color::Black);
    }

    #[test]
    fn test_get_color_with_single_mod() {
        // Arrange
        let single_mod_node = GojoNode {
            color: Color::Black,
            mods: Vec::from([Mod::new(ModData::Col(Color::Red), 2)]),
            ..Default::default()
        };
        let version = 2;

        // Act
        let ptr = NodePtr::<i32, i32>::from(single_mod_node);

        // Assert
        assert_eq!(ptr.get_color(version), Color::Red);
    }

    #[test]
    fn test_get_color_with_five_mods() {
        // Arrange
        let five_mod_node = GojoNode {
            color: Color::Black,
            mods: Vec::from([
                Mod::new(ModData::Col(Color::Red), 2),
                Mod::new(ModData::Col(Color::Black), 3),
                Mod::new(ModData::Col(Color::Red), 4),
                Mod::new(ModData::Col(Color::Black), 5),
                Mod::new(ModData::Col(Color::Red), 6),
            ]),
            ..Default::default()
        };
        let version = 6;

        // Act
        let ptr = NodePtr::<i32, i32>::from(five_mod_node);

        // Assert
        assert_eq!(ptr.get_color(version), Color::Red);
    }

    #[test]
    fn test_get_color_with_bursted_node() {
        let version = 7;
        let next_copy = GojoNode {
            version,
            color: Color::Black,
            ..Default::default()
        };
        let next_ptr = NodePtr::from(next_copy);
        let bursted_node = GojoNode {
            version: 1,
            color: Color::Black,
            mods: Vec::from([
                Mod::new(ModData::Col(Color::Red), 2),
                Mod::new(ModData::Col(Color::Black), 3),
                Mod::new(ModData::Col(Color::Red), 4),
                Mod::new(ModData::Col(Color::Black), 5),
                Mod::new(ModData::Col(Color::Red), 6),
            ]),
            next_copy: next_ptr,
            ..Default::default()
        };

        // Act
        let ptr = NodePtr::<i32, i32>::from(bursted_node);

        // Assert
        assert_eq!(ptr.get_color(version), Color::Black);
    }

    #[test]
    fn test_insert_increasing() {
        // Arrange
        let mut m = Gojo::new();
        let maximum = 10;

        // Act
        for key in 1..=maximum {
            m.insert(key, key << 2);
        }

        // Assert
        assert_eq!(unsafe { (*m.root.pointer).key }, 4);
        let expected = [
            (1, Color::Black),
            (2, Color::Black),
            (3, Color::Black),
            (4, Color::Black),
            (5, Color::Black),
            (6, Color::Black),
            (7, Color::Black),
            (8, Color::Red),
            (9, Color::Black),
            (10, Color::Red),
        ];
        for (key, color) in expected.iter() {
            let ptr = m.find_node(key, maximum);
            assert!(!ptr.is_null());
            unsafe {
                assert_eq!((*ptr.pointer).key, *key);
                assert_eq!(ptr.get_color(maximum), *color);
            }
        }
    }

    #[test]
    fn test_insert_decreasing() {
        // Arrange
        let mut m = Gojo::new();
        let maximum = 10;

        // Act
        for key in (1..=maximum).rev() {
            m.insert(key, key << 2);
        }

        // Assert
        assert_eq!(unsafe { (*m.root.pointer).key }, 7);
        let expected = [
            (1, Color::Red),
            (2, Color::Black),
            (3, Color::Red),
            (4, Color::Black),
            (5, Color::Black),
            (6, Color::Black),
            (7, Color::Black),
            (8, Color::Black),
            (9, Color::Black),
            (10, Color::Black),
        ];
        for (key, color) in expected.iter() {
            let ptr = m.find_node(key, maximum);
            assert!(!ptr.is_null());
            unsafe {
                assert_eq!((*ptr.pointer).key, *key);
                assert_eq!(ptr.get_color(maximum), *color);
            }
        }
    }

    #[test]
    fn test_cant_find_element_if_not_in_good_version() {
        // Arrange
        let mut m = Gojo::new();
        let upper_limit = 100;

        // Act
        for key in 1..=upper_limit {
            m.insert(key, key << 2);
        }

        // Assert
        for key in 1..upper_limit {
            assert!(m.get(&key, key - 1).is_none());
            assert!(m.get(&key, key).is_some());
            assert!(m.get(&(key + 1), key).is_none());
        }
    }

    #[test]
    #[ignore]
    fn test_remove_red_node() {
        // Arrange
        let mut m = Gojo::new();
        let maximum = 10;

        // Act
        for key in 1..=maximum {
            m.insert(key, key << 2);
        }
        let res = m.remove(&10);

        // Assert
        assert!(res.is_some());
    }

    #[test]
    #[ignore]
    fn test_remove() {
        // Arrange
        let mut m = Gojo::new();
        let maximum = 100;

        // Act
        for key in 1..=maximum {
            m.insert(key, key << 2);
        }
        let res = m.remove(&1);

        // Assert
        assert_eq!(res, Some(2));

        for key in 1..=maximum {
            assert!(!m.find_node(&key, maximum).is_null());
        }

        assert!(m.find_node(&1, maximum + 1).is_null());
    }
}
