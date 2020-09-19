use crate::ast;
use crate::Spanned;
use crate::{Parse, ParseError, Parser};

/// A visibility level restricted to some path: pub(self)
/// or pub(super) or pub(crate) or pub(in some::module).
#[derive(Debug, Clone)]
pub struct VisRestricted {
    /// The `pub` keyword.
    pub pub_: ast::Pub,
    /// `(` to specify the start of the visibility scope.
    pub open: ast::OpenParen,
    /// Optional `in` keyword when specifying a path scope.
    pub in_: Option<ast::In>,
    /// The path in which the `pub` applies.
    pub path: ast::Path,
    /// `)` to specify the end of the path.
    pub close: ast::CloseParen,
}

into_tokens!(VisRestricted {
    pub_,
    open,
    in_,
    path,
    close
});

impl Spanned for VisRestricted {
    fn span(&self) -> runestick::Span {
        self.pub_.span().join(self.close.span())
    }
}

impl Parse for VisRestricted {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(VisRestricted {
            pub_: parser.parse()?,
            open: parser.parse()?,
            in_: parser.parse()?,
            path: parser.parse()?,
            close: parser.parse()?,
        })
    }
}
