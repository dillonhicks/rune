use crate::{ast, ParseErrorKind};
use crate::{IntoTokens, Parse, ParseError, Parser, Peek, Resolve, Spanned, Storage};
use runestick::{Source, Span};
use std::borrow::Cow;

type PathSegments = Vec<(ast::Scope, ast::Ident)>;

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone)]
pub struct Path {
    /// The optional leading colon `::`
    pub leading_colon: Option<ast::Scope>,
    /// The first component in the path.
    pub first: ast::Ident,
    /// The rest of the components in the path.
    pub rest: PathSegments,
    /// Trailing scope.
    pub trailing: Option<ast::Scope>,
}

impl Path {
    /// Borrow as an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components.
    pub fn try_as_ident(&self) -> Option<&ast::Ident> {
        if self.rest.is_empty() && self.trailing.is_none() {
            Some(&self.first)
        } else {
            None
        }
    }

    /// Iterate over all components in path.
    pub fn into_components(&self) -> impl Iterator<Item = &'_ ast::Ident> + '_ {
        let mut first = Some(&self.first);
        let mut it = self.rest.iter();

        std::iter::from_fn(move || {
            if let Some(first) = first.take() {
                return Some(first);
            }

            Some(&it.next()?.1)
        })
    }
}

impl Spanned for Path {
    fn span(&self) -> Span {
        if let Some(trailing) = &self.trailing {
            return self.first.span().join(trailing.span());
        }

        if let Some((_, ident)) = self.rest.last() {
            return self.first.span().join(ident.span());
        }

        self.first.span()
    }
}

impl Peek for Path {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        matches!(t1.kind, ast::Kind::Ident(..))
    }
}
/// Parsing Paths
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, ParseError};
///
/// parse_all::<ast::Path>("x").unwrap();
/// parse_all::<ast::Path>("::x").unwrap();
/// parse_all::<ast::Path>("a::b").unwrap();
/// parse_all::<ast::Path>("::ab::cd").unwrap();
/// parse_all::<ast::Path>("crate").unwrap();
/// parse_all::<ast::Path>("super").unwrap();
/// parse_all::<ast::Path>("crate::foo").unwrap();
/// parse_all::<ast::Path>("super::bar").unwrap();
/// parse_all::<ast::Path>("::super").unwrap();
/// parse_all::<ast::Path>("::crate").unwrap();
/// ```
///
impl Parse for Path {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let leading_colon = parser.parse::<Option<ast::Scope>>()?;

        let token = parser.token_peek_eof()?;

        let first: ast::Ident = match token.kind {
            ast::Kind::Ident(_) => parser.parse()?,
            ast::Kind::Super => parser.parse::<ast::Super>()?.into(),
            ast::Kind::Crate => parser.parse::<ast::Crate>()?.into(),
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::TokenMismatch {
                        expected: ast::Kind::Ident(ast::StringSource::Text),
                        actual: token.kind,
                    },
                ))
            }
        };

        Ok(Self {
            leading_colon,
            first,
            rest: parser.parse()?,
            trailing: parser.parse()?,
        })
    }
}

impl<'a> Resolve<'a> for Path {
    type Output = Vec<Cow<'a, str>>;

    fn resolve(
        &self,
        storage: &Storage,
        source: &'a Source,
    ) -> Result<Vec<Cow<'a, str>>, ParseError> {
        let mut output = Vec::new();

        output.push(self.first.resolve(storage, source)?);

        for (_, ident) in &self.rest {
            output.push(ident.resolve(storage, source)?);
        }

        Ok(output)
    }
}

impl IntoTokens for Path {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.first.into_tokens(context, stream);

        for (sep, rest) in &self.rest {
            sep.into_tokens(context, stream);
            rest.into_tokens(context, stream);
        }
    }
}
