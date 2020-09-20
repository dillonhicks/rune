use crate::ast;
use crate::{Parse, Peek, Spanned, ToTokens};

/// `_` as an indication that the type should be
/// inferred by the compiler.
///
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
#[allow(missing_docs)]
pub struct TypeInfer {
    pub underscore: ast::Underscore,
}

impl Peek for TypeInfer {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Underscore)
    }
}
