use super::ast::AST;
use super::expr::expr;
use super::result::Span;
use crate::error::{Error, Result};

pub fn parse_query(input: &str) -> Result<AST> {
    let (rest, ex) = match expr(None)(Span::new(input)) {
        Ok((r, e)) => (r, e),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            return Err(Error::from(e.message()))
        }
        Err(nom::Err::Incomplete(_)) => unreachable!(),
    };

    assert!(rest.len() == 0);
    Ok(AST::new(ex))
}
