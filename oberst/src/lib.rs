use std::collections::HashMap;

pub mod parser;
pub use oberst_proc::define_command;

pub type Parse<Context> = for<'a> fn(
    &mut parser::CommandParser<'a>,
) -> Result<Execute<'a, Context>, parser::ParseError<'a>>;
pub type Execute<'a, Context> = Box<dyn FnOnce(&Context) -> CommandResult<'a>>;

pub enum CommandError<'a> {
    Parse(parser::ParseError<'a>),
    Dispatch(Box<dyn std::error::Error + 'a>),
}

pub type CommandResult<'a> = std::result::Result<i32, CommandError<'a>>;

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
    usage: &'static CommandUsage,
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
        usage: &'static CommandUsage,
        dispatchers: &'static [CommandDispatch<Context>],
    ) {
        assert!(!dispatchers.is_empty());
        debug_assert!(name.chars().all(char::is_alphabetic));
        self.commands.insert(name, Command { usage, dispatchers });
    }

    pub fn get_usage(&self, command: &str) -> Option<&CommandUsage> {
        self.commands.get(command).map(|command| command.usage)
    }

    pub fn dispatch<'a>(&'a self, command: &'a str) -> CommandResult {
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

#[macro_export]
macro_rules! register_command {
    ($source:expr, $name:ident) => {
        ($source).register(stringify!($name), $name::USAGE, $name::DISPATCHERS)
    };
}
