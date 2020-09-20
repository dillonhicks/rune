use crate::ast;
use crate::{Parse, Peek, Spanned, ToTokens};

/// The `...` type for `extern` function definitions
///
/// # Parsing Examples
///
/// ```
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::TypeVariadic>("...").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
#[allow(missing_docs)]
pub struct TypeVariadic {
    pub ellipsis: ast::Ellipsis,
}

impl Peek for TypeVariadic {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Ellipsis)
    }
}
