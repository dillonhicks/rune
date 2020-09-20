use crate::ast;
use crate::{Parse, Peek, Spanned, ToTokens};

/// The type of a function argument, assignment, or expression
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
pub struct TypeHint {
    /// The `:` token between ident and type
    pub colon: ast::Colon,
    /// The type of the argument
    pub type_: Box<ast::Type>,
}

impl Peek for TypeHint {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Colon) && ast::Type::peek(t2, None)
    }
}
