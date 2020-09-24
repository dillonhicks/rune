use crate::collections::HashMap;
use crate::{Assembly, CompileError, CompileErrorKind, CompileVisitor};
use crate::{CompileResult, Spanned};
use runestick::{Inst, SourceId, Span};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// The kind of scope
#[derive(Copy, Clone, Debug)]
pub(crate) enum PathKind {
    /// The crate scope
    Crate,
    /// A file scope
    File,
    /// A module scope
    Mod,
    /// A struct or enum body scope,
    StructOrEnumBody,
    /// An impl block scope
    Impl,
    /// A function scope
    Fn,
    /// A macro scope
    Macro,
    /// A closure scope
    Closure,
    /// An anonymous block scope
    Block,
    /// Marker
    Anon,
}

#[derive(Clone, Debug)]
pub(crate) struct PathPart {
    parent: Option<std::num::NonZeroUsize>,
    idx: usize,
    name: String,
    kind: PathKind,
}

impl PathPart {
    pub(crate) fn is_crate(&self) -> bool {
        if let PathKind::Crate = self.kind {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_struct_or_enum_body(&self) -> bool {
        if let PathKind::StructOrEnumBody = self.kind {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_file(&self) -> bool {
        if let PathKind::File = self.kind {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_closure(&self) -> bool {
        if let PathKind::Closure = self.kind {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_block(&self) -> bool {
        if let PathKind::Block = self.kind {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_function(&self) -> bool {
        if let PathKind::Fn = self.kind {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_module(&self) -> bool {
        if let PathKind::Mod = self.kind {
            true
        } else {
            false
        }
    }

    /// Get the name of the scope
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn kind(&self) -> PathKind {
        self.kind
    }

    pub(crate) fn parent(&self) -> Option<std::num::NonZeroUsize> {
        self.parent
    }

    pub(crate) fn idx(&self) -> usize {
        self.idx
    }
}

#[derive(Clone)]
pub(crate) struct PathRef {
    idx: usize,
    path_stack: PathStack,
}

impl PathRef {
    pub fn deref(&self) -> &PathPart {
        &self.path_stack[self.idx]
    }

    pub fn push_child<S: ToString>(&self, name: S, kind: PathKind) -> CompileResult<PathRef> {
        Ok(self.path_stack.push(idx, name, kind))
    }

    pub fn push_sibling<S: ToString>(&self, name: S, kind: PathKind) -> CompileResult<PathRef> {
        let parent = self.deref().parent().map(|n| n.get()).unwrap_or(0);
        Ok(self.path_stack.push(parent, name, kind))
    }

    pub fn parent(&self) -> Option<PathRef> {
        let parent_idx = self.deref().parent();
        parent_idx
            .map(|idx| idx.get())
            .map(|idx| self.path_stack.)
    }
}

impl fmt::Debug for PathRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.deref().fmt(f)
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    path_stack: Vec<PathPart>,
}

impl Inner {
    pub fn with_crate_name<S: ToString>(name: S) -> Self {
        Inner {
            path_stack: vec![PathPart {
                parent: None,
                idx: 0,
                name: name.to_string(),
                kind: PathKind::Crate,
            }],
        }
    }
}

#[derive(Clone, Debug)]
pub struct PathStack {
    inner: Rc<RefCell<Inner>>,
}

impl PathStack {
    pub fn with_crate_name<S: ToString>(name: S) -> Self {
        PathStack {
            inner: Rc::new(RefCell::new(Inner::with_crate_name(name))),
        }
    }

    /// Get the scope corresponding to `crate`
    pub(crate) fn crate_(&self) -> PathRef {
        PathRef {
            idx: 0,
            path_stack: self.clone(),
        }
    }

    /// Get the scope corresponding to `super`
    pub(crate) fn super_<S: Spanned>(&self, span: S) -> CompileResult<PathRef> {
        let idx = self
            .inner
            .borrow_mut()
            .path_stack
            .iter()
            .rev()
            .filter(PathPart::is_module)
            .take(2)
            .last()
            .map(PathPart::idx)
            .ok_or_else(|| {
                CompileError::unresolvable_path(span, "there are too many leading `super` keywords")
            })?;

        Ok(PathRef {
            idx,
            path_stack: self.clone(),
        })
    }

    pub(crate) fn get(&self, idx: usize) -> Option<PathRef> {
        self.inner
    }

    /// Get the scope corresponding to `self`, which should be the first module
    /// found when backwards walking the scopes.
    pub(crate) fn self_(&self) -> CompileResult<PathRef> {
        let idx = self
            .inner
            .borrow_mut()
            .path_stack
            .iter()
            .rev()
            .find(PathPart::is_module)
            .map(|part| part.idx())
            .ok_or_else(|| {
                CompileError::unresolvable_path(span, "cannot resolve `self`, there are no modules")
            })?;
        Ok(PathRef {
            idx,
            path_stack: self.clone(),
        })
    }

    /// Get the scope corresponding to `Self`
    pub(crate) fn self_type(&self) -> CompileResult<PathRef> {
        let idx = self
            .inner
            .borrow_mut()
            .path_stack
            .iter()
            .rev()
            .find(PathPart::is_struct_or_enum_body)
            .map(|part| part.idx())
            .ok_or_else(|| CompileError::unresolvable_path(span, "unresolved import `Self`"))?;

        Ok(PathRef {
            idx,
            path_stack: self.clone(),
        })
    }

    fn len(&self) -> usize {
        self.inner.borrow().path_stack.len()
    }

    fn push<S: ToString>(&self, parent: usize, name: S, kind: PathKind) -> PathRef {
        let idx = self.len();
        self.inner.borrow_mut().path_stack.push(PathPart {
            parent: std::num::NonZeroUsize::new(parent),
            idx,
            name: name.to_string(),
            kind,
        });

        PathRef {
            idx,
            path_stack: self.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::PathStack;

    #[test]
    fn test_path_stack() {
        let stack = PathStack::with_crate_name("foo");
        println!("{:?}", stack)
    }
}
