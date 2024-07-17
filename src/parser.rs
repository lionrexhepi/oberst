use std::fmt::{self, Display, Formatter};

/// Helper to parse command syntax.
pub struct CommandParser<'a> {
    command: &'a str,
    offset: usize,
}

impl<'a> CommandParser<'a> {
    /// Create a parser for the given command.
    pub fn new(command: &'a str) -> Self {
        Self { command, offset: 0 }
    }

    /// Match the given literal to the command, advancing the parser if successful.
    /// Returns an error if the literal does not match.
    pub fn lit(&mut self, lit: &str) -> Result<(), ParseError<'a>> {
        if self.command[self.offset..].starts_with(lit) {
            self.offset += lit.len();
            Ok(())
        } else {
            Err(self.error(ParseErrorKind::BadLiteral))
        }
    }

    /// Parse an argument of the given type.
    /// See the `Argument` trait for more information.
    pub fn argument<A: Argument>(&mut self) -> Result<A, ParseError<'a>> {
        A::parse(self)
    }

    /// Advance the parser by the given number of characters.
    pub fn advance(&mut self, n: usize) {
        self.offset += n;
    }

    /// Consume an arbitrary number of whitespace characters greater than one.
    pub fn spacing(&mut self) -> Result<(), ParseError<'a>> {
        let spacing = self.read_while(char::is_whitespace);
        if spacing.is_empty() {
            Err(self.error(ParseErrorKind::ExpectedWhitespace))
        } else {
            Ok(())
        }
    }

    /// Read characters from the command while the given predicate is true.
    /// Returns the read characters.
    pub fn read_while<F>(&mut self, mut f: F) -> &'a str
    where
        F: FnMut(char) -> bool,
    {
        let start = self.offset;
        while let Some(c) = self.command[self.offset..].chars().next() {
            if f(c) {
                self.offset += c.len_utf8();
            } else {
                break;
            }
        }
        &self.command[start..self.offset]
    }

    /// Generate and return a `ParseError` at the current position.   
    pub fn error(&self, kind: ParseErrorKind) -> ParseError<'a> {
        ParseError {
            command: self.command,
            offset: self.offset,
            kind,
        }
    }

    /// Expect the end of the command.
    pub fn end(&mut self) -> Result<(), ParseError<'a>> {
        if self.offset == self.command.len() {
            Ok(())
        } else {
            Err(self.error(ParseErrorKind::ExpectedEof))
        }
    }

    /// Create a copy of this parser at the current position.
    pub fn branch(&self) -> Self {
        Self {
            command: self.command,
            offset: self.offset,
        }
    }
}

/// An error that occurs during parsing.
#[derive(Debug)]
pub struct ParseError<'a> {
    command: &'a str,
    offset: usize,
    pub kind: ParseErrorKind,
}

impl Display for ParseError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let start = self.offset.saturating_sub(10);
        let end = self.offset + 10;
        let command = &self.command[start..end];
        match self.kind {
            ParseErrorKind::UnknownCommand => write!(f, "Unknown command: `{}`", command),
            ParseErrorKind::UnexpectedEof => write!(f, "Unexpected end of command"),
            ParseErrorKind::ExpectedEof => write!(f, "Expected end of command"),
            ParseErrorKind::BadArgument => write!(f, "Bad argument"),
            ParseErrorKind::BadLiteral => write!(f, "Bad literal"),
            ParseErrorKind::ExpectedWhitespace => write!(f, "Expected whitespace"),
        }
    }
}

impl std::error::Error for ParseError<'_> {}

#[derive(Debug)]
pub enum ParseErrorKind {
    /// The given command name has no command associated with it.
    UnknownCommand,
    /// The parser expected further input, though none was found.
    UnexpectedEof,
    /// The parser expected the end of the command, but found more input.
    ExpectedEof,
    /// The parser failed to process an argument.
    BadArgument,
    /// The parser failed to match a literal.
    BadLiteral,
    /// The parser expected whitespaces
    ExpectedWhitespace,
}

/// A trait for parsing arguments from a command.
/// This trait is implemented for most basic types, though it is possible to implement it for custom types as well.
pub trait Argument {
    fn parse<'a>(parser: &mut CommandParser<'a>) -> Result<Self, ParseError<'a>>
    where
        Self: Sized;
}

impl Argument for () {
    fn parse<'a>(_: &mut CommandParser<'a>) -> Result<Self, ParseError<'a>> {
        Ok(())
    }
}

impl Argument for String {
    fn parse<'a>(parser: &mut CommandParser<'a>) -> Result<Self, ParseError<'a>>
    where
        Self: Sized,
    {
        parser.lit("\"")?;
        let mut result = String::new();
        let mut escape = false;
        parser.read_while(|c| {
            if escape {
                result.push(c);
                escape = false;
                true
            } else if c == '\\' {
                escape = true;
                true
            } else if c == '"' {
                false
            } else {
                result.push(c);
                true
            }
        });
        parser.lit("\"")?;
        Ok(result.to_string())
    }
}

/// Helper macro to conditionally generate code based on a boolean parameter.
macro_rules! cond {
    (if true  { $($t:tt)* } else { $($e:tt)*}) => {
        $($t)*
    };
    (if false  { $($t:tt)* } else { $($e:tt)*}) => {
        $($e)*
    };
    (if $other:tt { $($t:tt)* } else { $($e:tt)*}) => {
       compile_error!("Conditional parameter must be either true or false");
    };
}

/// Implement the `Argument` trait for a list of integer types.
/// Uses a boolean parameter with the `cond!` macro to conditionally generate code for signed integers instead of having two separate macro definitons.
macro_rules! argument_impl_int {
    ($signed: ident, $($t:ty),*) => {
        $(
            impl Argument for $t {
                fn parse<'a>(parser: &mut CommandParser<'a>) -> Result<Self, ParseError<'a>> {
                    let sign = cond! {
                        if $signed {
                            if parser.command[parser.offset..].starts_with('-') {
                                parser.advance(1);
                                -1
                            } else {
                                1
                            }
                        } else {
                            1
                        }
                    };
                    let num = parser.read_while(|c| c.is_ascii_digit() );
                    if num.is_empty() {
                        Err(parser.error(ParseErrorKind::BadArgument))
                    } else {
                        Ok(num.parse::<$t>().map_err(|_| parser.error(ParseErrorKind::BadArgument))? * sign)
                    }
                }
            }
        )*
    };
}

macro_rules! argument_impl_float {
    ($($t:ty),*) => {
        $(
            impl Argument for $t {
                fn parse<'a>(parser: &mut CommandParser<'a>) -> Result<Self, ParseError<'a>> {
                    let sign = if parser.command[parser.offset..].starts_with('-') {
                        parser.advance(1);
                        -1.0
                    } else {
                        1.0
                    };
                    let mut decimals = false;
                    let num = parser.read_while(|c|  {
                        if c == '.' {
                            if decimals  {
                                false
                            } else {
                                decimals = true;
                                true
                            }
                        } else {
                            c.is_ascii_digit()
                        }
                    });
                    if num.is_empty() {
                        Err(parser.error(ParseErrorKind::BadArgument))
                    } else {
                        Ok(num.parse::<$t>().map_err(|_| parser.error(ParseErrorKind::BadArgument))? * sign)
                    }
                }
            }
        )*
    };
}

argument_impl_int!(false, u8, u16, u32, u64, u128, usize);
argument_impl_int!(true, i8, i16, i32, i64, i128, isize);
argument_impl_float!(f32, f64);
