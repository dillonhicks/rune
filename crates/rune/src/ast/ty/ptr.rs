use crate::ast;
use crate::{Parse, Peek, Spanned, ToTokens};

/// A raw pointer type: `*const T` or `*mut T`.
///
/// # Parsing Examples
///
/// ```
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::TypePtr>("*mut X").unwrap();
/// parse_all::<ast::TypePtr>("*const Y").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
#[allow(missing_docs)]
pub struct TypePtr {
    pub star: ast::Mul,
    pub mutability: Option<ast::Mutability>,
    pub elem: Box<ast::Type>,
}

impl Peek for TypePtr {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Star)
            && (ast::Mutability::peek(t2, None) || ast::Type::peek(t2, None))
    }
}
