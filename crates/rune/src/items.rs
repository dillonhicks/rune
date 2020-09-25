use crate::path_tree::{PathId, PathKind, PathRef, PathTree, PathTreeError};
use crate::sec;
use crate::worker::QualifiedPath;
use runestick::{Component, IntoComponent, Item};
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

pub(super) struct Guard {
    path: Rc<RefCell<Vec<Node>>>,
    tree_guard: Option<crate::path_tree::Guard>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let exists = self.path.borrow_mut().pop().is_some();
        debug_assert!(exists);
    }
}

#[derive(Debug, Clone)]
struct Node {
    children: usize,
    component: Component,
}

impl From<Component> for Node {
    fn from(component: Component) -> Self {
        Self {
            children: 0,
            component,
        }
    }
}

/// Manage item paths.
#[derive(Clone, Debug)]
pub(super) struct Items {
    path: Rc<RefCell<Vec<Node>>>,
    tree: PathTree,
}

impl Items {
    /// Construct a new items manager.
    pub fn new(base: Vec<Component>) -> Self {
        let tree = PathTree::empty();
        let mut path = vec![];
        for component in base.into_iter() {
            let x = match &component {
                Component::String(n) => tree.push_scoped(n, PathKind::Mod, sec::Public),
                Component::Block(n)
                | Component::Closure(n)
                | Component::AsyncBlock(n)
                | Component::Macro(n) => tree.push_scoped(n, PathKind::Block, sec::None),
            }
            .expect("could not push node to tree");

            std::mem::forget(x);

            path.push(Node {
                children: 0,
                component,
            })
        }

        Self {
            path: Rc::new(RefCell::new(path)),
            tree,
        }
    }

    pub fn crate_(&self) -> String {
        self.tree.crate_().name()
    }

    pub fn super_(&self) -> Result<QualifiedPath, PathTreeError> {
        self.tree.super_().map(|supe| supe.qualified_path())
    }

    pub fn self_(&self) -> Result<QualifiedPath, PathTreeError> {
        self.tree.self_().map(|me| me.qualified_path())
    }

    /// Take a snapshot of the existing items.
    pub fn snapshot(&self) -> Self {
        Self {
            path: Rc::new(RefCell::new(self.path.borrow().clone())),
            tree: PathTree::cloned(&self.tree),
        }
    }

    pub fn find(&self, qualpath: &QualifiedPath) -> Result<PathRef, PathTreeError> {
        self.tree.find(qualpath)
    }

    pub(crate) fn current(&self) -> PathRef {
        self.tree.current()
    }

    pub(crate) fn get(&self, idx: usize) -> Option<PathRef> {
        self.tree.get(idx)
    }

    /// Check if the current path is empty.
    pub fn is_empty(&self) -> bool {
        self.path.borrow().is_empty()
    }

    /// Get the next child id.
    fn next_child(&mut self) -> usize {
        let mut path = self.path.borrow_mut();

        if let Some(node) = path.last_mut() {
            let new = node.children + 1;
            mem::replace(&mut node.children, new)
        } else {
            0
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_block(&mut self) -> Guard {
        let index = self.next_child();

        self.path
            .borrow_mut()
            .push(Node::from(Component::Block(index)));

        let kind = PathKind::Block;
        let guard = self
            .tree
            .push_scoped(index, kind, sec::None)
            .unwrap_or_else(|err| panic!("unable to push {:?} {} to tree: {}", kind, index, err));

        Guard {
            path: self.path.clone(),
            tree_guard: Some(guard),
        }
    }

    /// Push a closure component and return guard associated with it.
    pub fn push_closure(&mut self) -> Guard {
        let index = self.next_child();

        self.path
            .borrow_mut()
            .push(Node::from(Component::Closure(index)));

        let kind = PathKind::Closure;
        let guard = self
            .tree
            .push_scoped(index, kind, sec::None)
            .unwrap_or_else(|err| panic!("unable to push {:?} {} to tree: {}", kind, index, err));

        Guard {
            path: self.path.clone(),
            tree_guard: Some(guard),
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_async_block(&mut self) -> Guard {
        let index = self.next_child();

        self.path
            .borrow_mut()
            .push(Node::from(Component::AsyncBlock(index)));

        let kind = PathKind::Block;
        let guard = self
            .tree
            .push_scoped(index, kind, sec::None)
            .unwrap_or_else(|err| panic!("unable to push {:?} {} to tree: {}", kind, index, err));

        Guard {
            path: self.path.clone(),
            tree_guard: Some(guard),
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_macro(&mut self) -> Guard {
        let index = self.next_child();

        self.path
            .borrow_mut()
            .push(Node::from(Component::Macro(index)));

        Guard {
            path: self.path.clone(),
            tree_guard: None,
        }
    }

    /// Get the item for the current state of the path.
    pub fn item(&self) -> Item {
        let path = self.path.borrow();
        Item::of(path.iter().map(|n| &n.component))
    }

    /// Pop the last component.
    pub fn pop(&self) -> Option<Component> {
        let mut path = self.path.borrow_mut();
        Some(path.pop()?.component)
    }

    /// Push a module
    pub fn push_mod(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Mod, vis)
    }

    /// push a fn
    pub fn push_fn(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Fn, vis)
    }

    /// push a const def
    pub fn push_const(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Const, vis)
    }

    /// push a struct def
    pub fn push_struct(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Struct, vis)
    }

    /// push an enum
    pub fn push_enum(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Enum, vis)
    }

    /// push a struct field
    pub fn push_field(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Field, vis)
    }

    /// Push an impl
    pub fn push_impl(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Impl, vis)
    }

    /// push an enum variant
    pub fn push_variant(&mut self, name: &str, vis: sec::Visibility) -> Guard {
        self.push_named_kind(name, PathKind::Variant, vis)
    }

    pub(crate) fn print_tree(&self) {
        println!("{}", self.tree.tree_formatter());
    }

    fn push_named_kind(&mut self, name: &str, kind: PathKind, vis: sec::Visibility) -> Guard {
        self.path.borrow_mut().push(Node::from(Component::String(
            name.to_owned().into_boxed_str(),
        )));

        let guard = self
            .tree
            .push_scoped(name, kind, vis)
            .unwrap_or_else(|err| panic!("unable to push {:?} {} to tree: {}", kind, name, err));

        self.print_tree();

        Guard {
            path: self.path.clone(),
            tree_guard: Some(guard),
        }
    }
}
