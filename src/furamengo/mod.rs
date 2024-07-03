use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    fmt::Debug,
    rc::Rc,
};

const MAX_VERSION: usize = 6;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Color {
    Red,
    Black,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModKind {
    Left { value: Rc<RefCell<Node>> },
    Right { value: Rc<RefCell<Node>> },
    Parent { value: Rc<RefCell<Node>> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Mod {
    kind: ModKind,
    version: u8,
}

impl Mod {
    fn new(kind: ModKind, version: u8) -> Self {
        Mod { kind, version }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    color: Color,
    key: i32,
    left: Option<Rc<RefCell<Node>>>,
    right: Option<Rc<RefCell<Node>>>,
    parent: Option<Rc<RefCell<Node>>>,
    mods: Vec<Mod>,
}

impl Node {
    pub fn new(key: i32) -> Self {
        Node {
            color: Color::Red,
            key,
            left: None,
            right: None,
            parent: None,
            mods: Vec::with_capacity(6),
        }
    }

    pub fn change_value(&mut self, version: u8, mod_kind: ModKind) -> Option<Rc<RefCell<Node>>> {
        let new_mod = Mod::new(mod_kind, version);
        let mut has_new_root: Option<Rc<RefCell<Node>>> = Option::None;

        // Normal case
        if self.mods.len() != MAX_VERSION {
            self.mods.push(new_mod);
            return has_new_root;
        }

        // Overflow
        let new_node = Rc::new(RefCell::new(self.create_node_from_self()));

        match &new_node.borrow().parent {
            None => {
                has_new_root = Some(new_node.clone());
            }
            Some(v) => {
                let mut parent = (**v).borrow_mut();
                if let Some(left) = &parent.left {
                    if left.borrow().key == self.key {
                        parent.change_value(
                            version,
                            ModKind::Left {
                                value: new_node.clone(),
                            },
                        );
                    }
                } else {
                    parent.change_value(
                        version,
                        ModKind::Right {
                            value: new_node.clone(),
                        },
                    );
                }
            }
        }

        // Change left
        if let Some(v) = &(*new_node).borrow().left {
            (**v).borrow_mut().change_value(
                version,
                ModKind::Parent {
                    value: new_node.clone(),
                },
            );
        }

        // Change right
        if let Some(v) = &(*new_node).borrow().right {
            (**v).borrow_mut().change_value(
                version,
                ModKind::Parent {
                    value: new_node.clone(),
                },
            );
        }

        has_new_root
    }

    fn create_node_from_self(&self) -> Node {
        let mut node = Node {
            color: self.color,
            key: self.key,
            left: self.left.clone(),
            right: self.right.clone(),
            parent: self.parent.clone(),
            mods: Vec::with_capacity(6),
        };

        for m in self.mods.iter() {
            match &m.kind {
                ModKind::Left { value } => {
                    node.left = Some(value.clone());
                }
                ModKind::Right { value } => {
                    node.right = Some(value.clone());
                }
                ModKind::Parent { value } => {
                    node.parent = Some(value.clone());
                }
            }
        }

        node
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Furamengo {
    null: Rc<RefCell<Node>>,
    roots: Vec<Rc<RefCell<Node>>>,
    version: u8,
}

impl Furamengo {
    pub fn new() -> Self {
        let null = Rc::new(RefCell::new(Node::new(0)));
        Furamengo {
            null,
            roots: Vec::with_capacity(100),
            version: 0,
        }
    }

    pub fn insert(&mut self, key: i32) {
        let new_node = Rc::new(RefCell::new(Node::new(key)));
        (*new_node).borrow_mut().left = Some(self.null.clone());
        (*new_node).borrow_mut().right = Some(self.null.clone());

        let mut y = Option::None;
        let mut x = self.get_latest_root();

        while x != self.null {
            y = Some(x.clone());
            let node_key = (*x).borrow().key;
            if (node_key) < key {
                let temp = (*x).borrow().right.clone().unwrap();
                x = temp;
            } else {
                let temp = (*x).borrow().left.clone().unwrap();
                x = temp;
            }
        }

        (*new_node).borrow_mut().parent = y.clone();
        if y == Option::None {
            self.roots.push(new_node.clone());
        } else {
            let y_key = (*y.clone().unwrap()).borrow().key;
            if key < y_key {
                self.insert_new_root_if_needed((*y.clone().unwrap()).borrow_mut().change_value(
                    self.version,
                    ModKind::Left {
                        value: new_node.clone(),
                    },
                ));
            } else {
                self.insert_new_root_if_needed((*y.clone().unwrap()).borrow_mut().change_value(
                    self.version,
                    ModKind::Right {
                        value: new_node.clone(),
                    },
                ));
            }
        }

        if let None = y.clone() {
            (*new_node).borrow_mut().color = Color::Black;
            return;
        }

        if let None = (*(y.unwrap())).borrow().parent {
            return;
        }

        self.insert_fixup(new_node.clone(), self.version);
    }

    fn get_latest_root(&self) -> Rc<RefCell<Node>> {
        self.roots[self.roots.len() - 1].clone()
    }

    fn insert_new_root_if_needed(&mut self, node: Option<Rc<RefCell<Node>>>) {
        match node {
            None => return,
            Some(v) => {
                self.roots.push(v.clone());
            }
        }
    }

    fn insert_fixup(&mut self, node: Rc<RefCell<Node>>, version: u8) {
        while node.borrow().parent.clone().unwrap().borrow().color == Color::Red {
        }

        let latest_root = self.get_latest_root();
        (*latest_root).borrow_mut().color = Color::Black;
    }
}
