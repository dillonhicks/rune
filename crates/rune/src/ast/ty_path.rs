use crate::ast;
use crate::{Ast, Parse, Peek, Spanned, TokenStream};

/// A path that represents a type
#[derive(Debug, Clone, Ast, Spanned, Parse)]
pub struct TypePath {
    #[allow(missing_docs)]
    pub path: ast::Path,
}

impl Peek for TypePath {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::Path::peek(t1, t2)
    }
}
