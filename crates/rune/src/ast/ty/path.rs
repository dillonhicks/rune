use crate::ast;
use crate::{Parse, Peek, Spanned, ToTokens};

/// A path that represents a type
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
pub struct TypePath {
    #[allow(missing_docs)]
    pub path: ast::Path,
}

impl Peek for TypePath {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        ast::Path::peek(t1, _t2)
    }
}
