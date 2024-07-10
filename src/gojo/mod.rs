use anyhow::Result;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::marker;
use std::ptr;
use std::ptr::drop_in_place;
use thiserror::Error;

pub mod cli;
pub mod parser;

const MAX_MODS: usize = 6;
const DEFAULT_MAX_OPS: usize = 100;

#[derive(Error, Debug)]
pub enum GojoError {
    #[error("the version `{0}` is not available")]
    UnknownVersion(String),
    #[error("cannot convert null node to NodeInfo")]
    ForbiddenConvertionToNodeInfo,
    #[error("unknown gojo error")]
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Color {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Mod<K: Ord + Clone + Default, V: Clone + Default> {
    data: ModData<K, V>,
    version: usize,
}

impl<K: Ord + Clone + Default, V: Clone + Default> Mod<K, V> {
    #[allow(dead_code)]
    fn new(data: ModData<K, V>, version: usize) -> Self {
        Self { data, version }
    }
}

#[derive(PartialEq, Eq, Clone)]
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

impl<K, V> Debug for GojoNode<K, V>
where
    K: Ord + Clone + Default + Debug,
    V: Clone + Default + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GojoNode")
            .field("color", &self.color)
            .field("key", &self.key)
            .field("value", &self.value)
            .field("version", &self.version)
            .finish()
    }
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

pub struct NodePtr<K: Ord + Clone + Default, V: Clone + Default> {
    pointer: *mut GojoNode<K, V>,
    null: bool,
}

impl<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> Debug for NodePtr<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.is_null() {
            true => f
                .debug_struct("NodePtr")
                .field("pointer", &"nilson")
                .field("null", &self.null)
                .finish(),
            false => unsafe {
                let res = &(*self.pointer);
                f.debug_struct("NodePtr")
                    .field("pointer", res)
                    .field("null", &self.null)
                    .finish()
            },
        }
    }
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
        unsafe {
            if other.is_null() && self.is_null() {
                return true;
            }
            if other.is_null() || self.is_null() {
                return false;
            }
            (*other.pointer).key == (*self.pointer).key
        }
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

    fn set_color(&mut self, color: Color, version: usize) {
        if self.is_null() {
            return;
        }

        let mut ptr = self.get_last_copy(version);

        let curr_color = ptr.get_color(version);
        if color == curr_color {
            return;
        }

        unsafe {
            let new_mod = ModData::Col(color);
            ptr.set_modification(new_mod, version);
        }
    }

    fn set_red_color(&mut self, version: usize) {
        self.set_color(Color::Red, version);
    }

    fn set_black_color(&mut self, version: usize) {
        self.set_color(Color::Black, version);
    }

    #[allow(dead_code)]
    fn get_value(&self) -> V {
        unsafe { (*self.pointer).value.clone() }
    }

    unsafe fn get_next_copy(&self) -> NodePtr<K, V> {
        (*self.pointer).next_copy
    }

    fn get_last_copy(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            return *self;
        }
        let mut caba = *self;
        unsafe {
            while !caba.get_next_copy().is_null() && caba.get_next_copy().version() <= version {
                caba = caba.get_next_copy();
            }
        }
        caba
    }

    pub fn get_color(&self, version: usize) -> Color {
        if self.is_null() {
            return Color::Black;
        }
        unsafe {
            let ptr = self.get_last_copy(version);
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
        let ptr = self.get_last_copy(version);
        let other = ptr.parent(version).left(version);
        other == ptr
    }

    fn is_right_child(&self, version: usize) -> bool {
        let ptr = self.get_last_copy(version);
        let other = ptr.parent(version).right(version);
        other == ptr
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

    unsafe fn set_modification(&mut self, mod_data: ModData<K, V>, version: usize) {
        if self.is_null() {
            match mod_data {
                ModData::Parent(p) => (*self.pointer).parent = p,
                ModData::Left(l) => (*self.pointer).left = l,
                ModData::Right(r) => (*self.pointer).right = r,
                ModData::Col(c) => (*self.pointer).color = c,
            }
            return;
        }

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
            return;
        }

        // Create a new node with all mods and the new change right here
        let mut new_gojo_node = (*self.pointer).clone_with_latest_mods();
        new_gojo_node.version = version;

        let new_node_ptr = NodePtr {
            pointer: Box::into_raw(Box::new(new_gojo_node)),
            null: false,
        };
        (*self.pointer).next_copy = new_node_ptr;

        match mod_data {
            ModData::Parent(p) => {
                (*new_node_ptr.pointer).parent = p;
                (*new_node_ptr.pointer).bk_ptr_parent = p;
            }
            ModData::Left(l) => {
                (*new_node_ptr.pointer).left = l;
                (*new_node_ptr.pointer).bk_ptr_left = l;
            }
            ModData::Right(r) => {
                (*new_node_ptr.pointer).right = r;
                (*new_node_ptr.pointer).bk_ptr_right = r;
            }
            ModData::Col(c) => (*new_node_ptr.pointer).color = c,
        }

        // Update left back pontairos
        let mut bk_ptr_left = (*new_node_ptr.pointer).bk_ptr_left;
        if !bk_ptr_left.is_null() && bk_ptr_left.is_left_child(version) {
            bk_ptr_left.set_parent(new_node_ptr, version);
        }

        // Update right back pontairos
        let mut bk_ptr_right = (*new_node_ptr.pointer).bk_ptr_right;
        if !bk_ptr_right.is_null() && bk_ptr_right.is_right_child(version) {
            bk_ptr_right.set_parent(new_node_ptr, version);
        }

        let mut bk_ptr_parent = (*new_node_ptr.pointer).bk_ptr_parent;
        // We got a new root boys
        if bk_ptr_parent.is_null() {
            return;
        }

        // Update parent back pontairos that can have a new root
        if *self == bk_ptr_parent.left(version) {
            bk_ptr_parent.set_left(new_node_ptr, version);
        } else if *self == bk_ptr_parent.right(version) {
            bk_ptr_parent.set_right(new_node_ptr, version);
        };
    }

    fn set_parent(&mut self, parent: NodePtr<K, V>, version: usize) {
        let mut ptr = self.get_last_copy(version);
        unsafe {
            let new_mod = ModData::Parent(parent);
            ptr.set_modification(new_mod, version);
        }
    }

    fn set_left(&mut self, left: NodePtr<K, V>, version: usize) {
        let mut ptr = self.get_last_copy(version);
        unsafe {
            let new_mod = ModData::Left(left);
            ptr.set_modification(new_mod, version);
        }
    }

    fn set_right(&mut self, right: NodePtr<K, V>, version: usize) {
        let mut ptr = self.get_last_copy(version);
        unsafe {
            let new_mod = ModData::Right(right);
            ptr.set_modification(new_mod, version);
        }
    }

    fn parent(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            unsafe {
                return (*self.pointer).parent;
            }
        }
        unsafe {
            let ptr = self.get_last_copy(version);
            let mut value = (*ptr.pointer).parent;
            for m in (*ptr.pointer).mods.iter() {
                if m.version > version {
                    break;
                }
                if let ModData::Parent(d) = m.data {
                    value = d;
                }
            }
            value.get_last_copy(version)
        }
    }

    fn left(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            return *self;
        }
        unsafe {
            let ptr = self.get_last_copy(version);
            let mut value = (*ptr.pointer).left;
            for m in (*ptr.pointer).mods.iter() {
                if m.version > version {
                    break;
                }
                if let ModData::Left(d) = m.data {
                    value = d;
                }
            }
            value.get_last_copy(version)
        }
    }

    fn right(&self, version: usize) -> NodePtr<K, V> {
        if self.is_null() {
            return *self;
        }
        unsafe {
            let ptr = self.get_last_copy(version);
            let mut value = (*ptr.pointer).right;
            for m in (*ptr.pointer).mods.iter() {
                if m.version > version {
                    break;
                }
                if let ModData::Right(d) = m.data {
                    value = d;
                }
            }
            value.get_last_copy(version)
        }
    }

    fn null() -> NodePtr<K, V> {
        NodePtr {
            pointer: ptr::null_mut(),
            null: true,
        }
    }

    fn is_null(&self) -> bool {
        self.null
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

#[derive(Clone, Debug)]
pub struct NodeInfo<K: Ord + Default + Clone + Debug, V: Default + Clone + Debug> {
    pub depth: usize,
    pub node_ptr: NodePtr<K, V>,
    pub color: Color,
    pub key: K,
    pub value: V,
}

impl<K, V> NodeInfo<K, V>
where
    K: Ord + Clone + Default + Debug,
    V: Clone + Default + Debug,
{
    #[allow(unused)]
    pub fn new(depth: usize, key: K, value: V, color: Color) -> NodeInfo<K, V> {
        let node_ptr = NodePtr::null();
        Self {
            depth,
            node_ptr,
            color,
            key,
            value,
        }
    }

    fn from_node_ptr(node_ptr: NodePtr<K, V>, depth: usize, version: usize) -> NodeInfo<K, V> {
        unsafe {
            let key = (*node_ptr.pointer).key.clone();
            let value = (*node_ptr.pointer).value.clone();
            let color = node_ptr.get_color(version);
            Self {
                depth,
                node_ptr,
                color,
                key,
                value,
            }
        }
    }

    fn successor(&mut self, version: usize) -> Option<NodeInfo<K, V>> {
        let mut depth = self.depth;
        let mut x = self.node_ptr;
        if !x.right(version).is_null() {
            x = x.right(version);
            depth += 1;
            while !x.left(version).is_null() {
                x = x.left(version);
                depth += 1;
            }
            return Some(NodeInfo::from_node_ptr(x, depth, version));
        }

        let mut y = x.parent(version);
        while !y.is_null() && x == y.right(version) {
            x = y;
            y = x.parent(version);
            depth -= 1;
        }

        if y.is_null() {
            return None;
        }

        depth -= 1;
        Some(NodeInfo::from_node_ptr(y, depth, version))
    }

    fn first_child(root: NodePtr<K, V>, version: usize) -> NodeInfo<K, V> {
        let mut depth = 0;
        if root.is_null() {
            return NodeInfo::from_node_ptr(NodePtr::null(), depth, version);
        }
        let mut temp = root;
        while !temp.left(version).is_null() {
            temp = temp.left(version);
            depth += 1;
        }
        NodeInfo::from_node_ptr(temp, depth, version)
    }

    fn last_child(root: NodePtr<K, V>, version: usize) -> NodeInfo<K, V> {
        let mut depth = 0;
        if root.is_null() {
            return NodeInfo::from_node_ptr(NodePtr::null(), depth, version);
        }

        let mut temp = root;
        while !temp.right(version).is_null() {
            temp = temp.right(version);
            depth += 1;
        }
        NodeInfo::from_node_ptr(temp, depth, version)
    }

    fn is_null(&self) -> bool {
        self.node_ptr.is_null()
    }

    fn next(&mut self, version: usize) -> Option<NodeInfo<K, V>> {
        self.successor(version)
    }
}

pub struct NodeInfoIter<'a, K: Ord + Default + Clone + Debug + 'a, V: Default + Clone + Debug + 'a>
{
    head: Option<NodeInfo<K, V>>,
    tail: Option<NodeInfo<K, V>>,
    len: usize,
    version: usize,
    _marker: marker::PhantomData<&'a ()>,
}

impl<'a, K: Ord + Clone + Default + Debug + 'a, V: Default + Clone + Debug + 'a> Clone
    for NodeInfoIter<'a, K, V>
{
    fn clone(&self) -> NodeInfoIter<'a, K, V> {
        NodeInfoIter {
            head: self.head.clone(),
            tail: self.tail.clone(),
            len: self.len,
            version: self.version,
            _marker: self._marker,
        }
    }
}

impl<'a, K: Ord + Default + Clone + Debug + 'a, V: Default + Clone + Debug + 'a> Iterator
    for NodeInfoIter<'a, K, V>
{
    type Item = NodeInfo<K, V>;

    fn next(&mut self) -> Option<NodeInfo<K, V>> {
        if self.len == 0 {
            return None;
        }

        self.head.as_ref()?;

        if let Some(value) = &self.head {
            let mut info = value.clone();
            self.head = info.next(self.version);
            self.len -= 1;
            return Some(info);
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

pub struct GojoIter<'a, K: Ord + Default + Clone + 'a, V: Default + Clone + 'a> {
    head: NodePtr<K, V>,
    tail: NodePtr<K, V>,
    len: usize,
    version: usize,
    _marker: marker::PhantomData<&'a ()>,
}

impl<'a, K: Ord + Clone + Default + 'a, V: Default + Clone + 'a> Clone for GojoIter<'a, K, V> {
    fn clone(&self) -> GojoIter<'a, K, V> {
        GojoIter {
            head: self.head,
            tail: self.tail,
            len: self.len,
            version: self.version,
            _marker: self._marker,
        }
    }
}

impl<'a, K: Ord + Default + Clone + 'a, V: Default + Clone + 'a> Iterator for GojoIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.len == 0 {
            return None;
        }

        if self.head.is_null() {
            return None;
        }

        let (k, v) = unsafe { (&(*self.head.pointer).key, &(*self.head.pointer).value) };
        self.head = self.head.next(self.version);
        self.len -= 1;
        Some((k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

#[derive(Debug)]
pub struct Gojo<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> {
    root: NodePtr<K, V>,
    len: usize,
    curr_version: usize,
    roots: Vec<(NodePtr<K, V>, usize)>,
    nil: NodePtr<K, V>,
}

impl<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> Default for Gojo<K, V> {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_OPS)
    }
}

unsafe impl<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> Send for Gojo<K, V> {}

unsafe impl<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> Sync for Gojo<K, V> {}

// Drop all owned pointers if the tree is dropped
impl<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> Drop for Gojo<K, V> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// If key and value are both impl Clone, we can call clone to get a copy.
impl<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> Clone for Gojo<K, V> {
    fn clone(&self) -> Gojo<K, V> {
        unsafe {
            let amount_of_ops = self.roots.len();
            let mut new = Gojo::new(amount_of_ops);
            new.root = self.root.deep_clone(self.curr_version);
            new.len = self.len;
            new
        }
    }
}

#[allow(dead_code)]
impl<K, V> Gojo<K, V>
where
    K: Ord + Clone + Default + Debug,
    V: Clone + Default + Debug,
{
    pub fn print_in_order(node: NodePtr<K, V>, version: usize) {
        if node.is_null() {
            return;
        }
        Self::print_in_order(node.left(version), version);
        println!("{:?}", node.get_last_copy(version));
        Self::print_in_order(node.right(version), version);
    }
}

impl<K: Ord + Clone + Default + Debug, V: Clone + Default + Debug> Gojo<K, V> {
    /// Creates an empty `RBTree`.
    pub fn new(predected_amount_of_ops: usize) -> Gojo<K, V> {
        let mut nil = NodePtr::new(K::default(), V::default());
        nil.null = true;
        unsafe {
            (*(nil.pointer)).color = Color::Black;
            (*(nil.pointer)).left = nil;
            (*(nil.pointer)).right = nil;
            (*(nil.pointer)).parent = nil;
        }
        let mut roots = Vec::with_capacity(predected_amount_of_ops);
        roots.push((nil, 0));
        Gojo {
            root: NodePtr::null(),
            len: 0,
            curr_version: 0,
            roots,
            nil,
        }
    }

    /// Returns the len of `RBTree`.
    pub fn len(&self, version: usize) -> Option<usize> {
        if version > self.latest_version() {
            return None;
        }

        let (_, length) = self.roots[version];
        Some(length)
    }

    pub fn latest_version(&self) -> usize {
        self.curr_version
    }

    pub fn get_root(&self, version: usize) -> NodePtr<K, V> {
        if version > self.latest_version() {
            return NodePtr::null();
        }

        self.roots[version].0
    }

    pub fn is_empty(&self, version: usize) -> bool {
        if version > self.latest_version() {
            return true;
        }
        let len = self.roots[version].1;
        len == 0
    }

    pub fn predecessor(&self, k: &K, version: usize) -> Option<&V> {
        let node = self.find_node(k, version);
        if node.is_null() {
            return None;
        }

        let succ = self.predecessor_helper(node, version);
        if succ.is_null() {
            return None;
        }

        unsafe { Some(&(*succ.pointer).value) }
    }

    fn predecessor_helper(&self, node: NodePtr<K, V>, version: usize) -> NodePtr<K, V> {
        let mut x = node;
        if !x.left(version).is_null() {
            return x.left(version).max_node(version);
        }

        let mut y = x.parent(version);
        while !y.is_null() && x.is_left_child(version) {
            x = y;
            y = x.parent(version);
        }
        y
    }

    pub fn successor_by_key(&self, k: &K, version: usize) -> Option<&V> {
        if version > self.latest_version() {
            return None;
        }

        let root = self.roots[version].0;
        let mut x = root;
        unsafe {
            loop {
                let next = match k.cmp(&(*x.pointer).key) {
                    Ordering::Less => x.left(version),
                    _ => x.right(version),
                };
                if next.is_null() {
                    break;
                }
                x = next;
            }
        }

        if x.is_null() {
            return None;
        }

        unsafe {
            if let Ordering::Less = k.cmp(&(*x.pointer).key) {
                return Some(&(*x.pointer).value);
            }
        }

        let succ = self.successor_by_node(x, version);
        if succ.is_null() {
            return None;
        }

        unsafe { Some(&(*succ.pointer).value) }
    }

    pub fn successor_by_node(&self, node: NodePtr<K, V>, version: usize) -> NodePtr<K, V> {
        let mut x = node;
        if !node.right(version).is_null() {
            return x.right(version).min_node(version);
        }

        let mut y = x.parent(version);
        while !y.is_null() && x.is_right_child(version) {
            x = y;
            y = x.parent(version);
        }
        y
    }

    unsafe fn left_rotate(&mut self, node: NodePtr<K, V>) {
        let version = self.curr_version;
        let mut caba = node;
        let mut temp = caba.right(version);
        caba.set_right(temp.left(version), version);

        if !temp.left(version).is_null() {
            temp.left(version).set_parent(caba, version);
        }

        temp.set_parent(caba.parent(version), version);
        if caba.parent(version).is_null() {
            self.root = temp;
        } else if caba.is_left_child(version) {
            caba.parent(version).set_left(temp, version);
        } else {
            caba.parent(version).set_right(temp, version);
        }

        temp.set_left(caba, version);
        caba.set_parent(temp, version);
    }

    unsafe fn right_rotate(&mut self, node: NodePtr<K, V>) {
        let version = self.curr_version;
        let mut caba = node;
        let mut temp = caba.left(version);
        caba.set_left(temp.right(version), version);

        if !temp.right(version).is_null() {
            temp.right(version).set_parent(caba, version);
        }

        temp.set_parent(caba.parent(version), version);
        if caba.parent(version).is_null() {
            self.root = temp;
        } else if caba.is_right_child(version) {
            caba.parent(version).set_right(temp, version);
        } else {
            caba.parent(version).set_left(temp, version);
        }

        temp.set_right(caba, version);
        caba.set_parent(temp, version);
    }

    unsafe fn insert_fixup(&mut self, node: NodePtr<K, V>) {
        let version = self.curr_version;
        let mut dude = node;
        while dude != self.root && dude.parent(version).is_red_color(version) {
            if dude.parent(version).is_left_child(version) {
                let mut uncle = dude.parent(version).parent(version).right(version);

                // Case 1
                if uncle.is_red_color(version) {
                    dude.parent(version).set_black_color(version);
                    uncle.set_black_color(version);
                    dude.parent(version).parent(version).set_red_color(version);
                    dude = dude.parent(version).parent(version);
                    continue;
                }

                // Case 2
                if dude.is_right_child(version) {
                    dude = dude.parent(version);
                    self.left_rotate(dude);
                }

                // Case 3
                dude.parent(version).set_black_color(version);
                dude.parent(version).parent(version).set_red_color(version);
                self.right_rotate(dude.parent(version).parent(version));
            } else {
                let mut uncle = dude.parent(version).parent(version).left(version);

                // Case 4
                if uncle.is_red_color(version) {
                    uncle.set_black_color(version);
                    dude.parent(version).set_black_color(version);
                    dude.parent(version).parent(version).set_red_color(version);
                    dude = dude.parent(version).parent(version);
                    continue;
                }

                // Case 5
                if dude.is_left_child(version) {
                    dude = dude.parent(version);
                    self.right_rotate(dude);
                }

                // Case 6
                dude.parent(version).set_black_color(version);
                dude.parent(version).parent(version).set_red_color(version);
                self.left_rotate(dude.parent(version).parent(version));
            }
        }
        let mut possible_new_root = self.root;
        possible_new_root.set_black_color(version);
        self.root = possible_new_root.get_last_copy(version);
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.len += 1;
        let new_length = self.len;
        let node = NodePtr::new(k, v);
        let mut y = NodePtr::null();
        let mut x = self.roots[self.curr_version].0;
        self.curr_version += 1;

        unsafe {
            (*node.pointer).version = self.curr_version;
        }

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
                    y.set_left(node, self.curr_version);
                }
                _ => {
                    y.set_right(node, self.curr_version);
                }
            };
        }

        unsafe {
            self.insert_fixup(node);
        }

        self.roots.push((self.root, new_length));
    }

    pub fn find_node(&self, k: &K, version: usize) -> NodePtr<K, V> {
        if version > self.curr_version {
            return NodePtr::null();
        }

        let root = self.roots[version].0;
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

    pub fn fast_clear(&mut self) {
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
        let new_length = self.len;
        self.curr_version += 1;
        self.roots.push((self.root, new_length));

        let key = unsafe { Some(self.delete(node).1) };
        self.root = self.root.get_last_copy(self.curr_version);
        self.roots[self.curr_version].0 = self.root.get_last_copy(self.curr_version);
        key
    }

    pub fn college_remove(&mut self, k: &K) -> Option<V> {
        let node = self.find_node(k, self.curr_version);
        if node.is_null() {
            self.curr_version += 1;
            self.roots.push((self.root, self.len));
            return None;
        }

        self.len -= 1;
        let new_length = self.len;
        self.curr_version += 1;
        self.roots.push((self.root, new_length));

        let key = unsafe { Some(self.delete(node).1) };
        self.root = self.root.get_last_copy(self.curr_version);
        self.roots[self.curr_version].0 = self.root.get_last_copy(self.curr_version);
        key
    }

    unsafe fn delete_fixup(&mut self, caba: NodePtr<K, V>) {
        let mut x = caba;
        let version = self.curr_version;

        while x != self.root && x.is_black_color(version) {
            if x.is_left_child(version) {
                let mut brother = x.parent(version).right(version);

                // Case 1
                if brother.is_red_color(version) {
                    brother.set_black_color(version);
                    x.parent(version).set_red_color(version);
                    self.left_rotate(x.parent(version));
                    brother = x.parent(version).right(version);
                }

                // Case 2
                if brother.left(version).is_black_color(version)
                    && brother.right(version).is_black_color(version)
                {
                    brother.set_red_color(version);
                    x = x.parent(version);
                }
                // Case 3
                else {
                    if brother.right(version).is_black_color(version) {
                        brother.left(version).set_black_color(version);
                        brother.set_red_color(version);
                        self.right_rotate(brother);
                        brother = x.parent(version).right(version);
                    }

                    // Case 4
                    brother.set_color(x.parent(version).get_color(version), version);
                    x.parent(version).set_black_color(version);
                    brother.right(version).set_black_color(version);
                    self.left_rotate(x.parent(version));
                    x = self.root;
                }
            } else {
                let mut brother = x.parent(version).left(version);

                // Case 1
                if brother.is_red_color(version) {
                    brother.set_black_color(version);
                    x.parent(version).set_red_color(version);
                    self.right_rotate(x.parent(version));
                    brother = x.parent(version).left(version);
                }

                // Case 2
                if brother.right(version).is_black_color(version)
                    && brother.left(version).is_black_color(version)
                {
                    brother.set_red_color(version);
                    x = x.parent(version);
                } else {
                    // Case 3
                    if brother.left(version).is_black_color(version) {
                        brother.right(version).set_black_color(version);
                        brother.set_red_color(version);
                        self.left_rotate(brother);
                        brother = x.parent(version).left(version);
                    }

                    // Case 4
                    brother.set_color(x.parent(version).get_color(version), version);
                    x.parent(version).set_black_color(version);
                    brother.left(version).set_black_color(version);
                    self.right_rotate(x.parent(version));
                    x = self.root;
                }
            }
        }

        x.set_black_color(version);
    }

    unsafe fn delete(&mut self, z: NodePtr<K, V>) -> (K, V) {
        let version = self.curr_version;

        let mut y = z;
        let mut x;
        let mut original_y_color = y.get_color(version);

        if z.left(version).is_null() {
            x = z.right(version);
            self.transplant(z, x);
        } else if z.right(version).is_null() {
            x = z.left(version);
            self.transplant(z, x)
        } else {
            y = self.successor_by_node(z, version);
            x = y.right(version);
            original_y_color = y.get_color(version);

            if y.parent(version) == x {
                x.set_parent(y, version);
            } else {
                self.transplant(y, x);
                y.set_right(z.right(version), version);
                y.right(version).set_parent(y, version);
            }

            self.transplant(z, y);
            y.set_left(z.left(version), version);
            z.left(version).set_parent(y, version);

            y.set_color(z.get_color(version), version);
        }

        if original_y_color == Color::Black {
            self.delete_fixup(x);
        }

        let key = (*y.pointer).key.clone();
        let value = (*y.pointer).value.clone();
        if y.pointer.is_null() {
            drop_in_place(y.pointer);
        }
        (key, value)
    }

    fn transplant(&mut self, u: NodePtr<K, V>, other: NodePtr<K, V>) {
        let version = self.curr_version;
        let mut v = other;

        if u.parent(version).is_null() {
            self.root = v;
        } else if u.is_left_child(version) {
            u.parent(version).set_left(v, version)
        } else {
            u.parent(version).set_right(v, version)
        }

        v.set_parent(u.parent(version), version);
    }

    fn first_child(&self, root: NodePtr<K, V>, version: usize) -> NodePtr<K, V> {
        if root.is_null() {
            NodePtr::null()
        } else {
            let mut temp = root;
            while !temp.left(version).is_null() {
                temp = temp.left(version);
            }
            temp
        }
    }

    fn last_child(&self, root: NodePtr<K, V>, version: usize) -> NodePtr<K, V> {
        if root.is_null() {
            NodePtr::null()
        } else {
            let mut temp = root;
            while !temp.right(version).is_null() {
                temp = temp.right(version);
            }
            temp
        }
    }

    pub fn iter(&self, version: usize) -> Result<GojoIter<K, V>> {
        if version > self.latest_version() {
            anyhow::bail!(GojoError::UnknownVersion(format!("{version}")));
        }
        let root = self.roots[version].0;
        let len = self.roots[version].1;
        Ok(GojoIter {
            head: self.first_child(root, version),
            tail: self.last_child(root, version),
            len,
            version,
            _marker: marker::PhantomData,
        })
    }

    pub fn node_info_iter(&self, version: usize) -> Result<NodeInfoIter<K, V>> {
        if version > self.latest_version() {
            anyhow::bail!(GojoError::UnknownVersion(format!("{version}")));
        }
        let root = self.roots[version].0;
        let len = self.roots[version].1;
        let head = Some(NodeInfo::first_child(root, version));
        let tail = Some(NodeInfo::last_child(root, version));

        Ok(NodeInfoIter {
            head,
            tail,
            len,
            version,
            _marker: marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod tree_tests {
    use pretty_assertions::assert_eq;

    use crate::gojo::{Color, Mod, ModData, NodeInfo, NodePtr};

    use super::{Gojo, GojoNode};
    use anyhow::Result;

    #[test]
    fn test_get_color_without_mods() {
        // Arrange
        let no_mods_node = GojoNode {
            color: Color::Black,
            ..Default::default()
        };
        let ptr = NodePtr::<i32, i32>::from(no_mods_node);
        let expect_color = Color::Black;
        let version = 1;

        // Act
        let actual_color = ptr.get_color(version);

        // Assert
        assert_eq!(expect_color, actual_color);
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
        let ptr = NodePtr::<i32, i32>::from(five_mod_node);
        let expected_color = Color::Red;
        let version = 6;

        // Act
        let actual_color = ptr.get_color(version);

        // Assert
        assert_eq!(expected_color, actual_color);
    }

    #[test]
    fn test_get_color_with_bursted_node() {
        // Arrange
        let version = 7;
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
            ..Default::default()
        };
        let mut bursted_node_ptr = NodePtr::<i32, i32>::from(bursted_node);

        // Act
        bursted_node_ptr.set_color(Color::Black, version);
        let actual_color = bursted_node_ptr.get_color(version);

        // Assert
        assert_eq!(actual_color, Color::Black);
    }

    #[test]
    fn test_get_left_without_mods() {
        // Arrange
        let no_mods_node = GojoNode {
            ..Default::default()
        };
        let ptr = NodePtr::<i32, i32>::from(no_mods_node);
        let version = 1;

        // Act
        let actual_left = ptr.left(version);

        // Assert
        assert!(actual_left.is_null());
    }

    #[test]
    fn test_get_left_with_five_mods() {
        // Arrange
        let left = GojoNode {
            ..Default::default()
        };
        let expected_left = NodePtr::<i32, i32>::from(left);
        let five_mods_node = GojoNode {
            mods: Vec::from([
                Mod::new(ModData::Left(NodePtr::null()), 2),
                Mod::new(ModData::Left(NodePtr::null()), 3),
                Mod::new(ModData::Left(NodePtr::null()), 4),
                Mod::new(ModData::Left(expected_left), 5),
            ]),
            ..Default::default()
        };
        let ptr = NodePtr::<i32, i32>::from(five_mods_node);
        let version = 5;

        // Act
        let actual_left = ptr.left(version);

        // Assert
        assert_eq!(expected_left, actual_left);
    }

    #[test]
    fn test_get_left_with_bursted_node() {
        // Arrange
        let left_node = GojoNode {
            version: 7,
            ..Default::default()
        };
        let expected_left_ptr = NodePtr::<i32, i32>::from(left_node);
        let bursted_node = GojoNode {
            mods: Vec::from([
                Mod::new(ModData::Left(NodePtr::null()), 2),
                Mod::new(ModData::Left(NodePtr::null()), 3),
                Mod::new(ModData::Left(NodePtr::null()), 4),
                Mod::new(ModData::Left(NodePtr::null()), 4),
                Mod::new(ModData::Left(NodePtr::null()), 5),
                Mod::new(ModData::Left(NodePtr::null()), 6),
            ]),
            ..Default::default()
        };
        let mut bursted_node_ptr = NodePtr::<i32, i32>::from(bursted_node);
        let version = 7;

        // Act
        bursted_node_ptr.set_left(expected_left_ptr, version);
        let actual_left = bursted_node_ptr.left(version);

        // Assert
        assert_eq!(expected_left_ptr, actual_left);
    }

    #[test]
    fn test_get_right_without_mods() {
        // Arrange
        let no_mods_node = GojoNode {
            ..Default::default()
        };
        let ptr = NodePtr::<i32, i32>::from(no_mods_node);
        let version = 1;

        // Act
        let actual_right = ptr.right(version);

        // Assert
        assert!(actual_right.is_null());
    }

    #[test]
    fn test_get_right_with_five_mods() {
        // Arrange
        let right = GojoNode {
            ..Default::default()
        };
        let expected_right = NodePtr::<i32, i32>::from(right);
        let five_mods_node = GojoNode {
            mods: Vec::from([
                Mod::new(ModData::Right(NodePtr::null()), 2),
                Mod::new(ModData::Right(NodePtr::null()), 3),
                Mod::new(ModData::Right(NodePtr::null()), 4),
                Mod::new(ModData::Right(expected_right), 5),
            ]),
            ..Default::default()
        };
        let ptr = NodePtr::<i32, i32>::from(five_mods_node);
        let version = 5;

        // Act
        let actual_right = ptr.right(version);

        // Assert
        assert_eq!(expected_right, actual_right);
    }

    #[test]
    fn test_get_right_with_bursted_node() {
        // Arrange
        let right_node = GojoNode {
            version: 7,
            ..Default::default()
        };
        let expected_right_ptr = NodePtr::<i32, i32>::from(right_node);
        let bursted_node = GojoNode {
            mods: Vec::from([
                Mod::new(ModData::Right(NodePtr::null()), 2),
                Mod::new(ModData::Right(NodePtr::null()), 3),
                Mod::new(ModData::Right(NodePtr::null()), 4),
                Mod::new(ModData::Right(NodePtr::null()), 4),
                Mod::new(ModData::Right(NodePtr::null()), 5),
                Mod::new(ModData::Right(NodePtr::null()), 6),
            ]),
            ..Default::default()
        };
        let mut bursted_node_ptr = NodePtr::<i32, i32>::from(bursted_node);
        let version = 7;

        // Act
        bursted_node_ptr.set_right(expected_right_ptr, version);
        let actual_right = bursted_node_ptr.right(version);

        // Assert
        assert_eq!(expected_right_ptr, actual_right);
    }

    #[test]
    fn test_get_parent_without_mods() {
        // Arrange
        let no_mods_node = GojoNode {
            ..Default::default()
        };
        let ptr = NodePtr::<i32, i32>::from(no_mods_node);
        let version = 1;

        // Act
        let actual_parent = ptr.parent(version);

        // Assert
        assert!(actual_parent.is_null());
    }

    #[test]
    fn test_get_parent_with_five_mods() {
        // Arrange
        let parent = GojoNode {
            ..Default::default()
        };
        let expected_parent = NodePtr::<i32, i32>::from(parent);
        let five_mods_node = GojoNode {
            mods: Vec::from([
                Mod::new(ModData::Parent(NodePtr::null()), 2),
                Mod::new(ModData::Parent(NodePtr::null()), 3),
                Mod::new(ModData::Parent(NodePtr::null()), 4),
                Mod::new(ModData::Parent(expected_parent), 5),
            ]),
            ..Default::default()
        };
        let ptr = NodePtr::<i32, i32>::from(five_mods_node);
        let version = 5;

        // Act
        let actual_parent = ptr.parent(version);

        // Assert
        assert_eq!(expected_parent, actual_parent);
    }

    #[test]
    fn test_get_parent_with_bursted_node() {
        // Arrange
        let parent_node = GojoNode {
            version: 7,
            ..Default::default()
        };
        let mut expected_parent_ptr = NodePtr::<i32, i32>::from(parent_node);
        let bursted_node = GojoNode {
            mods: Vec::from([
                Mod::new(ModData::Parent(NodePtr::null()), 2),
                Mod::new(ModData::Parent(NodePtr::null()), 3),
                Mod::new(ModData::Parent(NodePtr::null()), 4),
                Mod::new(ModData::Parent(NodePtr::null()), 4),
                Mod::new(ModData::Parent(NodePtr::null()), 5),
                Mod::new(ModData::Parent(NodePtr::null()), 6),
            ]),
            ..Default::default()
        };
        let mut bursted_node_ptr = NodePtr::<i32, i32>::from(bursted_node);
        let version = 7;

        // Act
        expected_parent_ptr.set_left(bursted_node_ptr, version);
        bursted_node_ptr.set_parent(expected_parent_ptr, version);
        let actual_parent = bursted_node_ptr.parent(version);

        // Assert
        assert_eq!(expected_parent_ptr, actual_parent);
    }

    #[test]
    fn test_bursted_root_is_setted_as_new_root() {
        // Arrange
        let mut gojo = Gojo::<i32, i32>::default();
        let version = 7;

        // Act
        gojo.insert(5, 5);
        gojo.insert(1, 1);
        gojo.insert(8, 8);
        gojo.insert(3, 3);
        gojo.insert(2, 2);
        gojo.insert(6, 6);
        gojo.insert(7, 7);

        // Assert
        assert_eq!(gojo.root.get_value(), 5);
        assert_eq!(gojo.root.get_color(version), Color::Black);
        assert_eq!(gojo.root.version(), 7);
    }

    #[test]
    fn test_insert_increasing() {
        // Arrange
        let mut m = Gojo::default();
        let expected = [
            (1, Color::Black),
            (2, Color::Black),
            (3, Color::Black),
            (4, Color::Black),
            (5, Color::Black),
            (6, Color::Red),
            (7, Color::Black),
            (8, Color::Black),
            (9, Color::Black),
            (10, Color::Red),
            (11, Color::Black),
            (12, Color::Red),
        ];
        let maximum = expected.len();

        // Act
        for key in 1..=maximum {
            m.insert(key, key << 2);
        }

        // Assert
        assert_eq!(unsafe { (*m.root.pointer).key }, 4);
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
        let mut m = Gojo::default();
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
        let mut m = Gojo::default();
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
    fn test_remove_red_node() {
        // Arrange
        let mut m = Gojo::default();
        let maximum = 10;

        // Act
        for key in 1..=maximum {
            m.insert(key, key << 2);
        }
        let res = m.remove(&10);

        // Assert
        assert_eq!(res, Some(10 << 2));
        assert_eq!(Some(9), m.len(11));
        assert!(m.get(&10, 10).is_some());
        assert_eq!(m.latest_version(), 11);
        assert!(m.get(&10, 11).is_none());
    }

    #[test]
    fn test_remove_black_node() {
        // Arrange
        let mut m = Gojo::default();
        let maximum = 10;

        // Act
        for key in 1..=maximum {
            m.insert(key, key << 2);
        }
        let res = m.remove(&9);

        // Assert
        assert_eq!(res, Some(9 << 2));
        assert_eq!(m.get(&9, 10), Some(&(9 << 2)));
        assert_eq!(Some(9), m.len(11));
        assert_eq!(11, m.latest_version());
        assert!(m.find_node(&9, m.latest_version()).is_null());
    }

    #[test]
    fn test_empty_remove() {
        // Arrange
        let mut m: Gojo<isize, bool> = Gojo::default();

        // Act
        let res = m.remove(&0);

        // Assert
        assert_eq!(None, res);
        assert_eq!(Some(0), m.len(0));
        assert_eq!(0, m.latest_version());
    }

    #[test]
    fn test_is_empty() {
        // Arrange
        let mut m = Gojo::default();

        // Act
        m.insert(1, 2);

        // Assert
        assert!(!m.is_empty(1));
        assert!(m.remove(&1).is_some());
        assert!(m.is_empty(2));
    }

    #[test]
    fn test_successor_up_on_tree() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();
        let expected_succ = &2;
        let version = 3;

        // Act
        gojo.insert(1, 1);
        gojo.insert(2, 2);
        gojo.insert(3, 3);

        let succ = gojo.successor_by_key(&1, version);

        // Assert
        assert!(succ.is_some());
        assert_eq!(expected_succ, succ.unwrap());
    }

    #[test]
    fn test_successor_in_right_tree() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();
        let expected_succ = &3;
        let version = 3;

        // Act
        gojo.insert(1, 1);
        gojo.insert(2, 2);
        gojo.insert(3, 3);

        let succ = gojo.successor_by_key(&2, version);

        // Assert
        assert!(succ.is_some());
        assert_eq!(expected_succ, succ.unwrap());
    }

    #[test]
    fn test_no_successor() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();
        let version = 3;

        // Act
        gojo.insert(1, 1);
        gojo.insert(2, 2);
        gojo.insert(3, 3);

        let succ = gojo.successor_by_key(&3, version);

        // Assert
        assert!(succ.is_none());
    }

    #[test]
    fn test_successor_in_different_versions() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();

        // Act
        gojo.insert(1, 1);
        gojo.insert(2, 2);
        gojo.insert(3, 3);

        let version_two_succ = gojo.successor_by_key(&2, 2);
        let version_tree_succ = gojo.successor_by_key(&2, 3);

        // Assert
        assert!(version_two_succ.is_none());
        assert!(version_tree_succ.is_some());
        assert_eq!(&3, version_tree_succ.unwrap());
    }

    #[test]
    fn test_predecessor_left_tree() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();
        let expected_pred = &1;
        let version = 3;

        // Act
        gojo.insert(1, 1);
        gojo.insert(2, 2);
        gojo.insert(3, 3);

        let pred = gojo.predecessor(&2, version);

        // Assert
        assert!(pred.is_some());
        assert_eq!(expected_pred, pred.unwrap());
    }

    #[test]
    fn test_predecessor_up_on_tree() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();
        let expected_pred = &2;
        let version = 3;

        // Act
        gojo.insert(1, 1);
        gojo.insert(2, 2);
        gojo.insert(3, 3);

        let pred = gojo.predecessor(&3, version);

        // Assert
        assert!(pred.is_some());
        assert_eq!(expected_pred, pred.unwrap());
    }

    #[test]
    fn test_no_predecessor() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();
        let version = 3;

        // Act
        gojo.insert(1, 1);
        gojo.insert(2, 2);
        gojo.insert(3, 3);

        let pred = gojo.predecessor(&1, version);

        // Assert
        assert!(pred.is_none());
    }

    #[test]
    fn test_predecessor_in_different_versions() {
        // Arrange
        let mut gojo: Gojo<usize, i32> = Gojo::default();

        // Act
        gojo.insert(2, 2);
        gojo.insert(3, 3);
        gojo.insert(1, 1);

        let version_two_succ = gojo.predecessor(&2, 2);
        let version_tree_succ = gojo.predecessor(&2, 3);

        // Assert
        assert!(version_two_succ.is_none());
        assert!(version_tree_succ.is_some());
        assert_eq!(&1, version_tree_succ.unwrap());
    }

    #[test]
    fn test_multiple_inserts_and_deletions() {
        // Arrange
        let mut gojo: Gojo<usize, usize> = Gojo::default();

        // Act
        for key in 1..=10 {
            gojo.insert(key, key);
        }

        for key in 1..=10 {
            gojo.remove(&key);
        }

        // Assert
        for key in 1..=10 {
            assert!(gojo.get(&key, 10).is_some());
        }

        for key in 1..=10 {
            assert!(gojo.get(&key, 20).is_none());
        }
    }

    #[test]
    fn test_len() {
        // Arrange
        let mut gojo: Gojo<usize, usize> = Gojo::default();

        // Act
        for key in 1..=10 {
            gojo.insert(key, key);
        }

        // Assert
        for version in 1..=10 {
            assert_eq!(Some(version), gojo.len(version));
        }
    }

    #[test]
    #[ignore]
    fn test_college_remove_that_changes_version_when_element_not_found() {
        todo!();
    }

    #[test]
    fn test_gojo_iterator() -> Result<()> {
        // Arrange
        let mut gojo: Gojo<usize, usize> = Gojo::default();

        // Act
        for i in 1..=10 {
            gojo.insert(i, i << 1);
        }
        let iter_to_version_five = gojo.iter(5)?;
        let iter_to_version_ten = gojo.iter(10)?;

        //Assert
        let mut counter = 0;
        for (k, v) in iter_to_version_five {
            assert_eq!(*v, *k << 1);
            counter += 1;
        }
        assert_eq!(5, counter);

        counter = 0;
        for (k, v) in iter_to_version_ten {
            assert_eq!(*v, *k << 1);
            counter += 1;
        }
        assert_eq!(10, counter);

        Ok(())
    }

    #[test]
    fn test_gojo_iterator_with_unknown_version() {
        // Arrange
        let mut gojo: Gojo<usize, usize> = Gojo::default();

        // Act
        for i in 1..=10 {
            gojo.insert(i, i << 1);
        }
        let iter_to_version_77 = gojo.iter(77);

        //Assert
        assert!(iter_to_version_77.is_err());
    }

    #[test]
    fn test_node_info_iterator() -> Result<()> {
        // Arrange
        let mut gojo: Gojo<usize, usize> = Gojo::default();
        let expected_cabas = [
            NodeInfo::new(2, 1, 1 << 1, Color::Black),
            NodeInfo::new(1, 2, 2 << 1, Color::Black),
            NodeInfo::new(2, 3, 3 << 1, Color::Black),
            NodeInfo::new(0, 4, 4 << 1, Color::Black),
            NodeInfo::new(2, 5, 5 << 1, Color::Black),
            NodeInfo::new(1, 6, 6 << 1, Color::Black),
            NodeInfo::new(3, 7, 7 << 1, Color::Black),
            NodeInfo::new(2, 8, 8 << 1, Color::Red),
            NodeInfo::new(3, 9, 9 << 1, Color::Black),
            NodeInfo::new(4, 10, 10 << 1, Color::Red),
        ];

        // Act
        for key in 1..=10 {
            gojo.insert(key, key << 1);
        }
        let mut iterator = gojo.node_info_iter(10)?;

        // Assert
        for expected in expected_cabas {
            let item = iterator.next();
            assert!(item.is_some());
            let actual = item.unwrap();
            assert_eq!(expected.key, actual.key);
            assert_eq!(expected.value, actual.value);
            assert_eq!(expected.color, actual.color);
            assert_eq!(expected.depth, actual.depth);
        }

        Ok(())
    }
}
