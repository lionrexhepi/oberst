use std::{any::Any, fmt::Debug};

use crate::arguments::CommandArgs;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MatchError<'a> {
    EndOfInput,
    InvalidInput(&'a str),
}

/// Used to validate input and extract arguments from it.
pub enum Checkpoint {
    Literal(String),
    Argument(String, Box<dyn Argument>),
    Wildcard,
}

impl Checkpoint {
    pub fn apply<'a>(
        &self,
        input: &'a str,
        args: &mut CommandArgs,
    ) -> Result<usize, MatchError<'a>> {
        match self {
            Checkpoint::Literal(lit) => {
                if input.len() == 0 {
                    Err(MatchError::EndOfInput)
                } else if input.starts_with(lit) {
                    Ok(lit.len())
                } else {
                    Err(MatchError::InvalidInput(&input[..lit.len()]))
                }
            }
            Checkpoint::Argument(name, value) => {
                let (advance, value) = value.apply(input)?;
                args.insert(name, value);
                Ok(advance)
            }
            Checkpoint::Wildcard => Ok(0),
        }
    }
}

pub trait Argument {
    fn apply<'a>(&self, input: &'a str) -> Result<(usize, Box<dyn Any>), MatchError<'a>>;
}

impl Argument for String {
    fn apply<'a>(&self, input: &'a str) -> Result<(usize, Box<dyn Any>), MatchError<'a>> {
        if input.starts_with('"') {
            let end = input[1..].find('"').ok_or(MatchError::EndOfInput)?;
            Ok((end + 2, Box::new(input[1..end].to_string())))
        } else {
            let end = input.find(' ').unwrap_or(input.len());
            Ok((end, Box::new(input[..end].to_string())))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_literals() {
        let matcher = Literal("foo".to_string());
        assert_eq!(matcher.apply("foo"), Ok(3));
        assert_eq!(matcher.apply("foo 123"), Ok(3));
        assert_eq!(matcher.apply("bar"), Err(MatchError::InvalidInput("bar")));
        assert_eq!(matcher.apply(""), Err(MatchError::EndOfInput))
    }
}
