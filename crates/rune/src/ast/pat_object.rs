use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;
/// An object pattern.
#[derive(Debug, Clone)]
pub struct PatObject {
    /// The identifier of the object pattern.
    pub ident: ast::LitObjectIdent,
    /// The open object marker.
    pub open: ast::OpenBrace,
    /// The items matched against.
    pub items: Vec<(PatObjectItem, Option<ast::Comma>)>,
    /// Indicates if the pattern is open or not.
    pub open_pattern: Option<ast::DotDot>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl PatObject {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        self.ident.span().join(self.close.span())
    }

    /// Parse the object with an opening path.
    pub fn parse_with_ident(
        parser: &mut Parser<'_>,
        ident: ast::LitObjectIdent,
    ) -> Result<Self, ParseError> {
        let open = parser.parse()?;
        let mut items = Vec::new();

        let mut is_open = true;

        while !parser.peek::<ast::CloseBrace>()? && !parser.peek::<ast::DotDot>()? {
            let item = parser.parse()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            is_open = comma.is_some();
            items.push((item, comma));

            if !is_open {
                break;
            }
        }

        let open_pattern = if is_open && parser.peek::<ast::DotDot>()? {
            Some(parser.parse()?)
        } else {
            None
        };

        let close = parser.parse()?;

        Ok(Self {
            ident,
            open,
            items,
            close,
            open_pattern,
        })
    }
}

impl Parse for PatObject {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let ident = parser.parse()?;
        Self::parse_with_ident(parser, ident)
    }
}

/// An object item.
#[derive(Debug, Clone)]
pub struct PatObjectItem {
    /// The key of an object.
    pub key: ast::LitObjectKey,
    /// The binding used for the pattern object.
    pub binding: Option<(ast::Colon, ast::Pat)>,
}

impl PatObjectItem {
    /// The span of the expression.
    pub fn span(&self) -> Span {
        if let Some((_, pat)) = &self.binding {
            self.key.span().join(pat.span())
        } else {
            self.key.span()
        }
    }
}

impl Parse for PatObjectItem {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let key = parser.parse()?;

        let binding = if parser.peek::<ast::Colon>()? {
            Some((parser.parse()?, parser.parse()?))
        } else {
            None
        };

        Ok(Self { key, binding })
    }
}
