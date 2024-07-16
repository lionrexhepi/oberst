use std::collections::HashMap;

pub mod parser;

pub type Parse<Context> = for<'a> fn(
    &mut parser::CommandParser<'a>,
) -> Result<Execute<'a, Context>, parser::ParseError<'a>>;
pub type Execute<'a, Context> = fn(&Context) -> Result<(), CommandError<'a>>;

pub enum CommandError<'a> {
    Parse(parser::ParseError<'a>),
    Dispatch(Box<dyn std::error::Error + 'a>),
}

impl<'a> From<parser::ParseError<'a>> for CommandError<'a> {
    fn from(error: parser::ParseError<'a>) -> Self {
        CommandError::Parse(error)
    }
}

impl<'a, E> From<E> for CommandError<'a>
where
    E: std::error::Error + 'a,
{
    fn from(error: E) -> Self {
        CommandError::Dispatch(Box::new(error))
    }
}

pub struct CommandUsage {
    pub name: &'static str,
    pub usage: &'static [&'static str],
    pub description: Option<&'static str>,
}

struct Command<Context: 'static> {
    usage: CommandUsage,
    dispatchers: &'static [CommandDispatch<Context>],
}

pub struct CommandDispatch<Context> {
    pub parser: Parse<Context>,
}

pub struct CommandSource<Context: 'static> {
    commands: HashMap<&'static str, Command<Context>>,
    context: Context,
}

impl<Context: 'static> CommandSource<Context> {
    pub fn new(context: Context) -> Self {
        Self {
            commands: HashMap::new(),
            context,
        }
    }

    pub fn register(
        &mut self,
        name: &'static str,
        description: &'static str,
        valid_forms: &'static [&'static str],
        dispatchers: &'static [CommandDispatch<Context>],
    ) {
        assert!(!dispatchers.is_empty());
        debug_assert!(name.chars().all(char::is_alphabetic));
        self.commands.insert(
            name,
            Command {
                usage: CommandUsage {
                    name,
                    usage: valid_forms,
                    description: Some(description),
                },
                dispatchers,
            },
        );
    }

    pub fn dispatch<'a>(&self, command: &'a str) -> Result<(), CommandError<'a>> {
        let mut parser = parser::CommandParser::new(command);
        let command = parser.read_while(|c| c.is_alphabetic());
        let command = self.commands.get(&command).ok_or(CommandError::Parse(
            parser.error(parser::ParseErrorKind::UnknownCommand),
        ))?;

        let mut last_error = None;

        for dispatch in command.dispatchers {
            let mut branch = parser.branch();
            match (dispatch.parser)(&mut branch) {
                Ok(execute) => {
                    return (execute)(&self.context);
                }
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }

        Err(CommandError::Parse(
            last_error.expect("Expected at least one dispatch"),
        ))
    }
}
