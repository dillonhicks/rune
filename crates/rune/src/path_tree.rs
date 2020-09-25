use crate::collections::HashMap;
use crate::sec;
use crate::worker::QualifiedPath;
use crate::{Assembly, CompileError, CompileErrorKind, CompileVisitor};
use crate::{CompileResult, Spanned};
use runestick::{Inst, SourceId, Span};
use std::borrow::{Borrow, Cow};
use std::cell::{Cell, Ref, RefCell};
use std::convert::TryFrom;
use std::fmt;
use std::rc::Rc;
use thiserror::Error;

type TreeUsize = u32;

/// Error when using the path tree
#[derive(Debug, Error)]
pub enum PathTreeError {
    /// Could not resolve a path due `msg`.
    #[error("failed to resolve: {msg}")]
    UnresolvablePath { msg: Cow<'static, str> },

    /// There are too many Path Parts in the path tree.
    ///
    #[error("too many paths: the number of paths would exceed the limit `{limit}`")]
    TooManyPaths { limit: TreeUsize },
}

impl PathTreeError {
    pub const fn too_many_paths() -> Self {
        Self::TooManyPaths {
            limit: TreeUsize::max_value(),
        }
    }
    pub fn unresolvable_path<S: Into<Cow<'static, str>>>(msg: S) -> Self {
        Self::UnresolvablePath { msg: msg.into() }
    }
}

/// The kind of scope
#[derive(Copy, Clone, Debug)]
pub(crate) enum PathKind {
    /// The root scope `::`.
    Package,
    /// The crate scope
    Crate,
    /// A file
    File,
    /// A module scope
    Mod,
    /// Use
    Use(PathId),
    /// A struct body
    Struct,
    /// An enum body
    Enum,
    /// type X = Y;
    TypeAlias,
    /// An impl block scope
    Impl,
    /// A function
    Fn,
    /// A const expression
    Const,
    /// A struct field
    Field,
    /// An enum variant
    Variant,
    /// A macro
    Macro,
    /// A closure
    Closure,
    /// An anonymous block
    Block,
}

impl PathKind {
    pub(crate) fn is_crate(&self) -> bool {
        if let PathKind::Crate = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_struct_or_enum(&self) -> bool {
        match self {
            PathKind::Struct | PathKind::Enum => true,
            _ => false,
        }
    }

    pub(crate) fn is_file(&self) -> bool {
        if let PathKind::File = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_closure(&self) -> bool {
        if let PathKind::Closure = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_block(&self) -> bool {
        if let PathKind::Block = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_function(&self) -> bool {
        if let PathKind::Fn = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_module(&self) -> bool {
        match self {
            PathKind::Mod | PathKind::Crate => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct PathId(TreeUsize);

impl PathId {
    pub const fn new(n: TreeUsize) -> Self {
        Self(n)
    }
    pub const fn to_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Debug for PathId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for PathId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PathPart {
    id: PathId,
    vis: sec::Visibility,
    parent: Option<PathId>,
    name: String,
    kind: PathKind,
    children: Vec<PathId>,
}

impl PathPart {
    pub(crate) fn visibility(&self) -> sec::Visibility {
        self.vis
    }

    /// Get the name of the scope
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn kind(&self) -> PathKind {
        self.kind
    }

    pub(crate) fn parent(&self) -> Option<PathId> {
        self.parent
    }

    pub(crate) fn id(&self) -> PathId {
        self.id
    }

    pub(crate) fn append_child(&mut self, id: PathId) {
        self.children.push(id)
    }
}

macro_rules! deref {
    ($self_:ident) => {
        &(RefCell::borrow(&*($self_).tree.inner).storage[$self_.idx])
    };
}

macro_rules! deref_mut {
    ($self_:ident) => {
        &mut (RefCell::borrow_mut(&*($self_).tree.inner).storage[$self_.idx])
    };
}

#[derive(Clone)]
pub(crate) struct PathRef {
    idx: usize,
    tree: PathTree,
}

impl std::cmp::PartialEq for PathRef {
    fn eq(&self, other: &Self) -> bool {
        self.idx.eq(&other.idx)
    }
}

impl PathRef {
    pub fn append_child<S: ToString>(
        &self,
        name: S,
        kind: PathKind,
        vis: sec::Visibility,
    ) -> Result<PathRef, PathTreeError> {
        let child = self.tree.push(self.idx, name, kind, vis)?;
        deref_mut!(self).append_child(PathId::new(child.idx as u32));
        Ok(child)
    }

    pub fn append_sibling<S: ToString>(
        &self,
        name: S,
        kind: PathKind,
    ) -> Result<PathRef, PathTreeError> {
        let parent_idx = deref!(self).parent().map(PathId::to_usize).unwrap_or(0);
        self.tree.push(parent_idx, name, kind, sec::None)
    }

    pub fn parent(&self) -> Option<PathRef> {
        let parent_idx = deref!(self).parent();
        parent_idx
            .map(PathId::to_usize)
            .filter(|idx| self.tree.get(*idx).is_some())
            .map(|idx| PathRef {
                idx,
                tree: self.tree.clone(),
            })
    }

    pub fn parent_mod(&self) -> Option<PathRef> {
        let mut node = self.clone();

        while let Some(parent) = node.parent() {
            if parent.kind().is_module() {
                return Some(parent);
            }

            node = parent.clone();
        }

        None
    }

    pub fn super_(&self) -> Option<PathRef> {
        self.parent_mod()
    }

    pub fn self_mod(&self) -> Option<PathRef> {
        let mut node = Some(self.clone());

        while let Some(next) = node {
            if next.kind().is_module() {
                return Some(next);
            }

            node = next.parent();
        }

        None
    }

    pub fn qualified_name(&self) -> String {
        self.qualified_path().to_string()
    }

    pub fn qualified_path(&self) -> QualifiedPath {
        let mut node = self.clone();
        let mut parts = vec![deref!(node).name().to_string()];

        while let Some(parent) = node.parent() {
            parts.push(deref!(parent).name().to_string());
            node = parent;
        }
        parts.reverse();
        parts.into()
    }

    pub fn name(&self) -> String {
        deref!(self).name().to_string()
    }

    pub fn kind(&self) -> PathKind {
        deref!(self).kind()
    }

    pub fn visibility(&self) -> sec::Visibility {
        deref!(self).visibility()
    }

    pub fn id(&self) -> PathId {
        deref!(self).id()
    }

    pub fn resolve(&self) -> PathRef {
        let mut node = self.clone();
        while let PathKind::Use(id) = node.kind() {
            let idx = id.to_usize();

            if idx == 0 {
                break;
            }

            node = PathRef {
                idx,
                tree: node.tree,
            }
        }

        node
    }

    pub fn iter_children(cloned: Self) -> impl Iterator<Item = PathRef> {
        let len = deref!(cloned).children.len();
        (0..len).filter_map(move |idx| {
            deref!(cloned)
                .children
                .get(idx)
                .cloned()
                .map(PathId::to_usize)
                .map(|idx| PathRef {
                    idx,
                    tree: cloned.tree.clone(),
                })
        })
    }
}

impl fmt::Debug for PathRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        deref!(self).fmt(f)
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    storage: Vec<PathPart>,
    current: usize,
}

impl Inner {
    pub fn with_crate_name<S: ToString>(name: S) -> Self {
        Inner {
            storage: vec![
                PathPart {
                    parent: None,
                    id: PathId::new(0),
                    name: String::new(),
                    kind: PathKind::Package,
                    children: vec![PathId::new(1)],
                    vis: sec::Public,
                },
                PathPart {
                    parent: Some(PathId::new(0)),
                    id: PathId::new(1),
                    name: name.to_string(),
                    kind: PathKind::Crate,
                    children: vec![],
                    vis: sec::Crate,
                },
            ],
            current: 1,
        }
    }

    pub fn empty() -> Self {
        Inner {
            storage: vec![PathPart {
                parent: None,
                id: PathId::new(0),
                name: String::new(),
                kind: PathKind::Package,
                children: vec![],
                vis: sec::Public,
            }],
            current: 0,
        }
    }
}

impl std::ops::Index<usize> for Inner {
    type Output = PathPart;

    fn index(&self, index: usize) -> &Self::Output {
        &self.storage[index]
    }
}

#[derive(Clone, Debug)]
pub struct PathTree {
    inner: Rc<RefCell<Inner>>,
}

impl PathTree {
    const PACKAGE_IDX: usize = 0;
    const CRATE_IDX: usize = 1;

    /// Construct a new path tree with a root module/crate name.
    pub fn with_crate_name<S: ToString>(name: S) -> Self {
        PathTree {
            inner: Rc::new(RefCell::new(Inner::with_crate_name(name))),
        }
    }

    /// Construct a new path tree with a root module/crate name.
    pub fn empty() -> Self {
        PathTree {
            inner: Rc::new(RefCell::new(Inner::empty())),
        }
    }

    pub(crate) fn get(&self, idx: usize) -> Option<PathRef> {
        (&*self.inner).borrow().storage.get(idx).map(|_| PathRef {
            idx,
            tree: self.clone(),
        })
    }

    /// Get the scope corresponding to `crate`
    pub(crate) fn crate_(&self) -> PathRef {
        PathRef {
            idx: Self::CRATE_IDX,
            tree: self.clone(),
        }
    }

    /// Get the scope corresponding to `super`
    pub(crate) fn super_(&self) -> Result<PathRef, PathTreeError> {
        self.self_()
            .ok()
            .and_then(|p| p.parent_mod())
            .ok_or_else(|| {
                PathTreeError::unresolvable_path("there are too many leading `super` keywords")
            })
    }

    /// Get the scope corresponding to `self`, which should be the first module
    /// found when backwards walking the scopes.
    pub(crate) fn self_(&self) -> Result<PathRef, PathTreeError> {
        self.current().self_mod().ok_or_else(|| {
            PathTreeError::unresolvable_path("could not determine context for `self`")
        })
    }

    /// Get the scope corresponding to `Self`
    pub(crate) fn self_type(&self) -> Result<PathRef, PathTreeError> {
        let idx = self
            .inner
            .borrow_mut()
            .storage
            .iter()
            .rev()
            .find(|p| p.kind().is_struct_or_enum())
            .map(PathPart::id)
            .map(PathId::to_usize)
            .ok_or_else(|| PathTreeError::unresolvable_path("unresolved import `Self`"))?;

        Ok(PathRef {
            idx,
            tree: self.clone(),
        })
    }

    /// The lenght of a thing
    fn len(&self) -> usize {
        (&*self.inner).borrow().storage.len()
    }

    fn push<S: ToString>(
        &self,
        parent_idx: usize,
        name: S,
        kind: PathKind,
        vis: sec::Visibility,
    ) -> Result<PathRef, PathTreeError> {
        let idx = self.len();
        let id = TreeUsize::try_from(idx)
            .map(PathId::new)
            .map_err(|_| PathTreeError::too_many_paths())?;
        let parent = TreeUsize::try_from(parent_idx)
            .map(PathId::new)
            .map_err(|_| PathTreeError::too_many_paths())?;

        self.inner.borrow_mut().storage.push(PathPart {
            parent: Some(parent),
            id,
            name: name.to_string(),
            kind,
            children: vec![],
            vis,
        });

        Ok(PathRef {
            idx,
            tree: self.clone(),
        })
    }

    pub(crate) fn push_scoped<S: ToString>(
        &self,
        name: S,
        kind: PathKind,
        vis: sec::Visibility,
    ) -> Result<Guard, PathTreeError> {
        let current_idx = (&*self.inner).borrow().current;

        let path_ref = PathRef {
            idx: current_idx,
            tree: self.clone(),
        }
        .append_child(name, kind, vis)?;

        self.inner.borrow_mut().current = path_ref.idx;
        Ok(Guard { path_ref })
    }

    pub fn fmt_list(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for path_ref in self.iter_refs() {
            writeln!(
                f,
                "{:<4} {:<10} {:<16} -> {}",
                path_ref.id(),
                format!("{:?}", path_ref.visibility()),
                format!("{:?}", path_ref.kind()),
                path_ref.qualified_name(),
            )?;
        }
        Ok(())
    }

    fn iter_refs(&self) -> impl Iterator<Item = PathRef> {
        let cloned = self.clone();
        let len = self.len();
        (0..len).map(move |idx| PathRef {
            idx,
            tree: cloned.clone(),
        })
    }

    fn pop(&self) -> Option<PathRef> {
        let idx: usize = (&*self.inner).borrow().current;
        let parent_idx = &(&*self.inner).borrow().storage[idx]
            .parent()
            .map(PathId::to_usize);
        match parent_idx {
            Some(parent_idx) => {
                self.inner.borrow_mut().current = *parent_idx;
                Some(PathRef {
                    idx,
                    tree: self.clone(),
                })
            }
            None => None,
        }
    }

    pub(crate) fn cloned(other: &Self) -> Self {
        PathTree {
            inner: Rc::new(RefCell::new(Inner {
                storage: (&*other.inner).borrow().storage.clone(),
                current: (&*other.inner).borrow().current,
            })),
        }
    }

    pub fn tree_formatter(&self) -> TreeFormatter<'_> {
        TreeFormatter(self)
    }

    pub(crate) fn find(&self, qualpath: &QualifiedPath) -> Result<PathRef, PathTreeError> {
        let mut q_iter = qualpath.iter().filter(|s| !s.is_empty());
        let mut current = self.get(0).unwrap();
        let mut last: Option<PathRef> = None;

        'depth: loop {
            let qualpart = if let Some(qualpart) = q_iter.next() {
                qualpart
            } else {
                break 'depth;
            };

            'breadth: for path_ref in PathRef::iter_children(current.clone()) {
                println!("{} == {}?", path_ref.name(), qualpart.as_str());
                if path_ref.name() == qualpart.as_str() {
                    current = path_ref;
                    last = Some(current.clone());
                    continue 'depth;
                } else {
                    last = None;
                    continue 'breadth;
                }
            }

            break;
        }

        if let Some(part) = q_iter.next() {
            return Err(PathTreeError::unresolvable_path(format!(
                "path resolution failed for {} at {}",
                qualpath, part
            )));
        }

        last.ok_or_else(|| {
            PathTreeError::unresolvable_path(format!("could not find path: {}", qualpath))
        })
    }

    pub(crate) fn current(&self) -> PathRef {
        PathRef {
            idx: (&*self.inner).borrow().current,
            tree: self.clone(),
        }
    }

    pub(crate) fn is_visible_to(
        &self,
        source: &QualifiedPath,
        target: &QualifiedPath,
    ) -> Result<bool, PathTreeError> {
        let target_path = self.find(target)?.resolve();
        let source_path = self.find(source)?.resolve();

        let is_visible = match target_path.visibility() {
            sec::None => false,
            sec::Crate => true,
            sec::Super => {
                let super_ = target_path.parent_mod().ok_or_else(|| {
                    PathTreeError::unresolvable_path(format!(
                        "could not resolve `super` of {}",
                        target
                    ))
                })?;

                let super_qualpath = super_.qualified_path();
                let ancestor_qualpath = source.common_ancestor(target);
                // let ancestor_ref = self.find(&ancestor_qualpath)?.resolve();

                (super_qualpath == ancestor_qualpath)
                    || super_qualpath.is_ancestor_of(&ancestor_qualpath)
            }
            sec::Public => true,
            sec::Private => {
                let target_self = target_path.self_mod();
                let source_self = source_path.self_mod();
                match (source_self, target_self) {
                    (Some(source), Some(target)) if source == target => true,
                    _ => false,
                }
            }
            sec::Inherit => false,
        };

        Ok(is_visible)
    }
}

pub(crate) struct Guard {
    path_ref: PathRef,
}

impl std::ops::Deref for Guard {
    type Target = PathTree;

    fn deref(&self) -> &Self::Target {
        &self.path_ref.tree
    }
}

impl std::ops::Drop for Guard {
    fn drop(&mut self) {
        if let Some(dropped) = self.path_ref.tree.pop() {
            assert_eq!(dropped.idx, self.path_ref.idx, "path tree was corrupted");
        } else {
            panic!("path tree was corrupted");
        }
    }
}

pub struct TreeFormatter<'a>(&'a PathTree);

impl fmt::Display for TreeFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt_list(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_storage() -> Result<(), Box<dyn std::error::Error>> {
        let tree = PathTree::with_crate_name("foo");
        let bar = tree
            .crate_()
            .append_child("bar", PathKind::Mod, sec::Super)?;
        for (ch, kind) in "ABCDEFGHI".chars().zip(
            [PathKind::Struct, PathKind::Enum]
                .into_iter()
                .copied()
                .cycle(),
        ) {
            bar.append_child(ch, kind, sec::Public)?;
        }
        let bar = bar.append_child("baz", PathKind::Mod, sec::Crate)?;
        for (vis, name) in [
            (sec::Private, "Alpha"),
            (sec::Private, "Beta"),
            (sec::Public, "Delta"),
            (sec::Public, "Gamma"),
        ]
        .into_iter()
        {
            bar.append_child(name, PathKind::Struct, *vis)?;
        }
        println!("{:?}", tree);

        println!("{}", TreeFormatter(&tree));
        Ok(())
    }
}
