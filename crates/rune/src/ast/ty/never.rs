use crate::ast;
use crate::{Parse, Peek, Spanned, ToTokens};

/// The `!` type
///
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
#[allow(missing_docs)]
pub struct TypeNever {
    pub bang: ast::Bang,
}

impl Peek for TypeNever {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Bang)
    }
}
