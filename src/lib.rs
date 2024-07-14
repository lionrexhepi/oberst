mod arguments;
pub(crate) mod matchers;

use arguments::CommandArgs;
use matchers::Checkpoint;
pub use matchers::MatchError;

#[derive(Debug)]
pub enum CommandError<'a> {
    InputError(MatchError<'a>),
    ExecError(Box<dyn std::error::Error + 'a>),
}

pub struct Dispatcher<C> {
    context: C,
    tree: CommandNode<C>,
}

impl<C> Dispatcher<C> {
    pub fn new(context: C) -> Self {
        Self {
            context,
            tree: CommandNode::root(),
        }
    }

    pub fn register(&mut self, node: CommandNode<C>) {
        self.tree.children.push(node)
    }

    pub fn dispatch<'a>(&mut self, command: &'a str) -> Result<(), CommandError<'a>> {
        self.tree.execute(&mut self.context, command.trim_start())
    }
}

pub struct CommandNode<C> {
    checkpoint: Checkpoint,
    // TODO: make sure two children cannot overlap: return an error/panic
    children: Vec<CommandNode<C>>,
    executes: Option<Box<dyn Fn(&mut C, &CommandArgs)>>,
}

impl<C> CommandNode<C> {
    pub fn root() -> Self {
        Self::new(Checkpoint::Wildcard)
    }

    fn new(checkpoint: Checkpoint) -> Self {
        Self {
            checkpoint,
            children: Vec::new(),
            executes: None,
        }
    }

    pub(crate) fn execute<'a>(
        &self,
        context: &mut C,
        mut command: &'a str,
    ) -> Result<(), CommandError<'a>> {
        // TODO: create only one shared instance of CommandArgs for each command
        let mut args = CommandArgs::new();
        let advance = self
            .checkpoint
            .apply(&command, &mut args)
            .map_err(CommandError::InputError)?;
        command = &command[advance..].trim_start();

        // Check for empty rest before excessively iterating over each child node
        if command.is_empty() {
            return self
                .executes
                .as_ref()
                .ok_or(CommandError::InputError(MatchError::EndOfInput))
                .map(|r| r(context, &args));
        }

        for child in &self.children {
            if let Ok(run) = child.execute(context, command) {
                return Ok(run);
            }
        }

        Err(CommandError::InputError(MatchError::InvalidInput(command)))
    }

    pub fn then(mut self, node: Self) -> Self {
        self.children.push(node);
        self
    }

    pub fn runs(mut self, f: impl Fn(&mut C, &CommandArgs) + 'static) -> Self {
        self.executes = Some(Box::new(f));
        self
    }
}

pub fn literal<C>(lit: impl ToString) -> CommandNode<C> {
    CommandNode::new(Checkpoint::Literal(lit.to_string()))
}

#[cfg(test)]
mod test {
    #[test]
    fn test_traversal() {
        use super::*;

        let mut dispatcher = Dispatcher::new(());
        dispatcher.register(
            literal("foo")
                .then(literal("bar").runs(|_, _| println!("foo bar")))
                .then(literal("baz").runs(|_, _| println!("foo baz"))),
        );
        dispatcher.register(literal("qux").runs(|_, _| println!("qux")));

        dispatcher.dispatch("foo bar").unwrap();
        dispatcher.dispatch("foo baz").unwrap();
        dispatcher.dispatch("foo").unwrap_err();
        dispatcher.dispatch("qux").unwrap();
    }
}
