use std::fmt::Debug;

/// Define conditions used to check whether a node's command tree should be followed.
pub trait Matcher: Debug {
    /// Check whether the conditions of this matcher are met with the given input slice.
    /// If the conditions are met, return the number of characters consumed.
    /// If the conditions are not met, return an error.
    fn apply<'a>(&self, input: &'a str) -> Result<usize, MatchError<'a>>;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MatchError<'a> {
    EndOfInput,
    InvalidInput(&'a str),
}

#[derive(Debug)]
pub struct Wildcard;

impl Matcher for Wildcard {
    fn apply<'a>(&self, _: &'a str) -> Result<usize, MatchError<'a>> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct Literal(pub String);

impl Matcher for Literal {
    fn apply<'a>(&self, input: &'a str) -> Result<usize, MatchError<'a>> {
        if input.len() == 0 {
            Err(MatchError::EndOfInput)
        } else if input.starts_with(&self.0) {
            Ok(self.0.len())
        } else {
            Err(MatchError::InvalidInput(&input[..self.0.len()]))
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
