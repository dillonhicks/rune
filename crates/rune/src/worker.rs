//! Worker used by compiler.

use crate::ast;
use crate::collections::HashMap;
use crate::const_compiler::Consts;
use crate::index::{Index as _, Indexer};
use crate::index_scopes::IndexScopes;
use crate::items::Items;
use crate::macros::MacroCompiler;
use crate::path_tree::{PathId, PathKind};
use crate::query::Query;
use crate::sec;
use crate::CompileResult;
use crate::{
    CompileError, CompileErrorKind, CompileVisitor, Errors, LoadError, MacroContext, Options,
    Resolve as _, SourceLoader, Sources, Spanned as _, Storage, UnitBuilder, Warnings,
};
use runestick::{Component, Context, Item, Source, SourceId, Span};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// A single task that can be fed to the worker.
#[derive(Debug)]
pub(crate) enum Task {
    /// Load a file.
    LoadFile {
        /// The kind of loaded file.
        kind: LoadFileKind,
        /// The item of the file to load.
        item: Item,
        /// The source id of the item being loaded.
        source_id: SourceId,
    },
    /// An indexing task, which will index the specified item.
    Index(Index),
    /// Task to process an import.
    Import(Import),
    /// Task to expand a macro. This might produce additional indexing tasks.
    ExpandMacro(Macro),
}

/// The kind of the loaded module.
#[derive(Debug)]
pub(crate) enum LoadFileKind {
    /// A root file, which determined a URL root.
    Root,
    /// A loaded module, which inherits its root from the file it was loaded
    /// from.
    Module { root: Option<PathBuf> },
}

#[derive(Debug)]
pub(crate) enum IndexAst {
    /// Index the root of a file with the given item.
    File(ast::File),
    /// Index an item.
    Item(ast::Item),
    /// Index a new expression.
    Expr(ast::Expr),
}

pub(crate) struct Worker<'a> {
    pub(crate) queue: VecDeque<Task>,
    context: &'a Context,
    pub(crate) sources: &'a mut Sources,
    options: &'a Options,
    pub(crate) errors: &'a mut Errors,
    pub(crate) warnings: &'a mut Warnings,
    pub(crate) visitor: &'a mut dyn CompileVisitor,
    pub(crate) source_loader: &'a mut dyn SourceLoader,
    pub(crate) query: Query,
    pub(crate) loaded: HashMap<Item, (SourceId, Span)>,
    pub(crate) expanded: HashMap<Item, Expanded>,
}

impl<'a> Worker<'a> {
    /// Construct a new worker.
    pub(crate) fn new(
        queue: VecDeque<Task>,
        context: &'a Context,
        sources: &'a mut Sources,
        options: &'a Options,
        unit: Rc<RefCell<UnitBuilder>>,
        consts: Rc<RefCell<Consts>>,
        errors: &'a mut Errors,
        warnings: &'a mut Warnings,
        visitor: &'a mut dyn CompileVisitor,
        source_loader: &'a mut dyn SourceLoader,
        storage: Storage,
    ) -> Self {
        Self {
            queue,
            context,
            sources,
            options,
            errors,
            warnings,
            visitor,
            source_loader,
            query: Query::new(storage, unit, consts),
            loaded: HashMap::new(),
            expanded: HashMap::new(),
        }
    }

    /// Run the worker until the task queue is empty.
    pub(crate) fn run(&mut self) {
        while let Some(task) = self.queue.pop_front() {
            match task {
                Task::LoadFile {
                    kind,
                    item,
                    source_id,
                } => {
                    log::trace!("load file: {}", item);

                    let source = match self.sources.get(source_id).cloned() {
                        Some(source) => source,
                        None => {
                            self.errors.push(LoadError::internal(
                                source_id,
                                "missing queued source by id",
                            ));

                            continue;
                        }
                    };

                    let file = match crate::parse_all::<ast::File>(source.as_str()) {
                        Ok(file) => file,
                        Err(error) => {
                            self.errors.push(LoadError::new(source_id, error));

                            continue;
                        }
                    };

                    let root = match kind {
                        LoadFileKind::Root => source.path().map(ToOwned::to_owned),
                        LoadFileKind::Module { root } => root,
                    };

                    let items = Items::new(item.clone().into_vec());

                    self.queue.push_back(Task::Index(Index {
                        root,
                        item,
                        items,
                        source_id,
                        source,
                        scopes: IndexScopes::new(),
                        impl_items: Default::default(),
                        ast: IndexAst::File(file),
                    }));
                }
                Task::Index(index) => {
                    let Index {
                        root,
                        item,
                        items,
                        source_id,
                        source,
                        scopes,
                        impl_items,
                        ast,
                    } = index;

                    log::trace!("index: {}", item);

                    let mut indexer = Indexer {
                        root,
                        storage: self.query.storage.clone(),
                        loaded: &mut self.loaded,
                        query: &mut self.query,
                        queue: &mut self.queue,
                        sources: self.sources,
                        source_id,
                        source,
                        warnings: self.warnings,
                        items,
                        scopes,
                        impl_items,
                        visitor: self.visitor,
                        source_loader: self.source_loader,
                    };

                    let result = match ast {
                        IndexAst::File(ast) => match indexer.index(&ast) {
                            Ok(()) => Ok(None),
                            Err(error) => Err(error),
                        },
                        IndexAst::Item(ast) => match indexer.index(&ast) {
                            Ok(()) => Ok(None),
                            Err(error) => Err(error),
                        },
                        IndexAst::Expr(ast) => match indexer.index(&ast) {
                            Ok(()) => Ok(Some(Expanded::Expr(ast))),
                            Err(error) => Err(error),
                        },
                    };

                    match result {
                        Ok(expanded) => {
                            if let Some(expanded) = expanded {
                                self.expanded.insert(item, expanded);
                            }
                        }
                        Err(error) => {
                            self.errors.push(LoadError::new(source_id, error));
                        }
                    }
                }
                Task::Import(import) => {
                    log::trace!("import: {}", import.item);

                    let source_id = import.source_id;

                    let result = import.process(
                        self.context,
                        &self.query.storage,
                        &mut *self.query.unit.borrow_mut(),
                    );

                    if let Err(error) = result {
                        self.errors.push(LoadError::new(source_id, error));
                    }
                }
                Task::ExpandMacro(m) => {
                    let Macro {
                        kind,
                        root,
                        items,
                        ast,
                        source,
                        source_id,
                        scopes,
                        impl_items,
                    } = m;

                    let item = items.item();
                    let span = ast.span();

                    log::trace!("expand macro: {} => {:?}", item, source.source(ast.span()));

                    match kind {
                        MacroKind::Expr => (),
                        MacroKind::Item => {
                            // NB: item macros are not expanded into the second
                            // compiler phase (only indexed), so we need to
                            // restore their item position so that indexing is
                            // done on the correct item.
                            match items.pop() {
                                Some(Component::Macro(..)) => (),
                                _ => {
                                    self.errors.push(
                                        LoadError::new(source_id, CompileError::internal(
                                            &span,
                                            "expected macro item as last component of macro expansion",
                                        ))
                                    );

                                    continue;
                                }
                            }
                        }
                    }

                    let mut macro_context =
                        MacroContext::new(self.query.storage.clone(), source.clone());

                    let mut compiler = MacroCompiler {
                        storage: self.query.storage.clone(),
                        item: item.clone(),
                        macro_context: &mut macro_context,
                        options: self.options,
                        context: self.context,
                        unit: self.query.unit.clone(),
                        source: source.clone(),
                    };

                    let ast = match kind {
                        MacroKind::Expr => {
                            let ast = match compiler.eval_macro::<ast::Expr>(ast) {
                                Ok(ast) => ast,
                                Err(error) => {
                                    self.errors.push(LoadError::new(source_id, error));

                                    continue;
                                }
                            };

                            IndexAst::Expr(ast)
                        }
                        MacroKind::Item => {
                            let ast = match compiler.eval_macro::<ast::Item>(ast) {
                                Ok(ast) => ast,
                                Err(error) => {
                                    self.errors.push(LoadError::new(source_id, error));

                                    continue;
                                }
                            };

                            IndexAst::Item(ast)
                        }
                    };

                    self.queue.push_back(Task::Index(Index {
                        root,
                        item,
                        items,
                        source_id,
                        source,
                        scopes,
                        impl_items,
                        ast,
                    }));
                }
            }
        }
    }
}

/// An item that has been expanded by a macro.
pub(crate) enum Expanded {
    /// The expansion resulted in an expression.
    Expr(ast::Expr),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QualifiedPath(Vec<String>);

impl QualifiedPath {
    pub fn new<S: ToString>(root: S) -> Self {
        QualifiedPath::from(vec![root.to_string()])
    }

    pub fn push(&mut self, s: String) {
        self.0.push(s)
    }

    pub fn pop(&mut self) -> Option<String> {
        self.0.pop()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.0.iter()
    }

    pub fn last(&self) -> Option<&'_ String> {
        self.0.last()
    }

    pub fn first(&self) -> Option<&'_ String> {
        self.0.first()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_ancestor_of(&self, other: &QualifiedPath) -> bool {
        (self.len() < other.len()) && (&self.0[..]) == (&other.0[..])
    }

    pub fn is_super_of(&self, other: &QualifiedPath) -> bool {
        (self.len() == (other.len() - 1)) && (&self.0[..]) == (&other.0[..])
    }

    pub fn common_ancestor(&self, other: &QualifiedPath) -> QualifiedPath {
        let parts = self
            .iter()
            .zip(other.iter())
            .take_while(|(a, b)| a.eq(b))
            .map(|(a, b)| a.clone())
            .collect::<Vec<_>>();

        QualifiedPath::from(parts)
    }
}

impl std::convert::From<Vec<String>> for QualifiedPath {
    fn from(inner: Vec<String>) -> Self {
        QualifiedPath(inner)
    }
}

impl fmt::Display for QualifiedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("::"))
    }
}

impl<'a> std::convert::From<&'a Item> for QualifiedPath {
    fn from(item: &Item) -> Self {
        let mut qualpath = QualifiedPath::default();

        for c in item.iter() {
            match c {
                Component::String(s) => {
                    qualpath.push(s.to_string());
                }
                Component::Block(idx)
                | Component::Closure(idx)
                | Component::AsyncBlock(idx)
                | Component::Macro(idx) => {
                    qualpath.push(idx.to_string());
                }
            }
        }

        qualpath
    }
}

impl std::iter::IntoIterator for QualifiedPath {
    type Item = String;
    type IntoIter = <Vec<String> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(mut self) -> Self::IntoIter {
        // TODO: This has to be here to to filter the empty root "":: path
        //   - or imports will fail.
        self.0.retain(|s| !s.is_empty());
        self.0.into_iter()
    }
}

/// Indexing to process.
#[derive(Debug)]
pub(crate) struct Index {
    /// The root URL of the file which caused this item to be indexed.
    root: Option<PathBuf>,
    /// Item being built.
    item: Item,
    /// Path to index.
    items: Items,
    /// The source id where the item came from.
    source_id: SourceId,
    /// The source where the item came from.
    source: Arc<Source>,
    scopes: IndexScopes,
    impl_items: Vec<Item>,
    ast: IndexAst,
}

/// Import to process.
#[derive(Debug)]
pub(crate) struct Import {
    pub(crate) item: Item,
    pub(crate) ast: ast::ItemUse,
    pub(crate) source: Arc<Source>,
    pub(crate) source_id: usize,
    pub(crate) qualified_path: QualifiedPath,
    pub(crate) items: Items,
    pub(crate) path_ref_id: usize,
}

impl Import {
    /// Process the import, populating the unit.
    pub(crate) fn process(
        self,
        context: &Context,
        storage: &Storage,
        unit: &mut UnitBuilder,
    ) -> CompileResult<()> {
        let Self {
            item,
            ast: decl_use,
            source,
            source_id,
            qualified_path,
            items,
            path_ref_id,
        } = self;

        let span = decl_use.span();

        let item_qualpath = QualifiedPath::from(&item);
        println!(">>>>>>> {:?}", item_qualpath);

        let item_ref = items.get(path_ref_id).expect("could not resolve path");
        println!(">>>>>>> {:?}", item_ref);

        let mut name = Item::of(QualifiedPath::into_iter(qualified_path.clone()));
        println!(">>>>>>> {:?}", name.iter().collect::<Vec<_>>());
        println!(">>>>>>> {:?}", qualified_path);

        if let Some((_, c)) = decl_use.rest.iter().next_back() {
            match c {
                ast::ItemUseComponent::Wildcard(..) => {
                    let mut new_names = Vec::new();

                    if !context.contains_prefix(&name) && !unit.contains_prefix(&name) {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::MissingModule { item: name },
                        ));
                    }

                    let iter = context
                        .iter_components(&name)
                        .chain(unit.iter_components(&name));

                    'components: for c in iter {
                        let mut qualpath = qualified_path.clone();
                        match &c {
                            Component::String(n) => {
                                qualpath.push(c.to_string());
                                let (vis, kind) = items
                                    .find(&qualpath)
                                    .map(|p| (p.visibility(), PathKind::Use(p.id())))
                                    .map_err(|_| {
                                        println!("Error Resolving: {:?}", qualpath);
                                    })
                                    .unwrap_or((sec::Public, PathKind::Use(PathId::new(0))));

                                if let sec::Private = vis {
                                    log::debug!("Skip import of {:?} item {}", vis, qualpath);
                                    continue 'components;
                                }

                                item_ref
                                    .append_child(
                                        qualpath.last().unwrap(),
                                        kind,
                                        (&decl_use.visibility).into(),
                                    )
                                    .expect("could not reslove path");
                            }
                            _ => {}
                        }

                        let mut name = name.clone();

                        name.push(c);
                        new_names.push(name);
                    }

                    for name in new_names {
                        unit.new_import(item.clone(), &name, span, source_id)?;
                    }
                    items.print_tree();
                }
                ast::ItemUseComponent::PathSegment(segment) => {
                    // let ident = segment
                    //     .try_as_ident()
                    //     .ok_or_else(|| CompileError::internal_unsupported_path(segment))?;
                    //
                    // let ident = ident.resolve(storage, &*source)?;

                    let kind = items
                        .find(&qualified_path)
                        .map(|p| PathKind::Use(p.id()))
                        .map_err(|_| {
                            println!("Error Resolving: {:?}", qualified_path);
                        })
                        .unwrap_or(PathKind::Use(PathId::new(0)));

                    item_ref
                        .append_child(
                            qualified_path.last().unwrap(),
                            kind,
                            (&decl_use.visibility).into(),
                        )
                        .expect("could not reslove path");
                    items.print_tree();
                    unit.new_import(item, &name, span, source_id)?;
                }
            }
        } else {
            let kind = items
                .find(&qualified_path)
                .map(|p| PathKind::Use(p.id()))
                .map_err(|_| {
                    println!("Error Resolving: {:?}", qualified_path);
                })
                .unwrap_or(PathKind::Use(PathId::new(0)));

            item_ref
                .append_child(
                    qualified_path.last().unwrap(),
                    kind,
                    (&decl_use.visibility).into(),
                )
                .expect("could not reslove path");
            items.print_tree();
            unit.new_import(item, &name, span, source_id)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MacroKind {
    Expr,
    Item,
}

#[derive(Debug)]
pub(crate) struct Macro {
    /// The kind of the macro.
    pub(crate) kind: MacroKind,
    /// The URL root at which the macro is being expanded.
    pub(crate) root: Option<PathBuf>,
    /// The item path where the macro is being expanded.
    pub(crate) items: Items,
    /// The AST of the macro call causing the expansion.
    pub(crate) ast: ast::MacroCall,
    /// The source where the macro is being expanded.
    pub(crate) source: Arc<Source>,
    /// The source id where the macro is being expanded.
    pub(crate) source_id: usize,
    /// Snapshot of index scopes when the macro was being expanded.
    pub(crate) scopes: IndexScopes,
    /// Snapshot of impl_items when the macro was being expanded.
    pub(crate) impl_items: Vec<Item>,
}
