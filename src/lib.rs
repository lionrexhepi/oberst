pub(crate) mod matchers;

use std::fmt::Debug;

pub use matchers::{MatchError, Matcher};

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

    pub fn dispatch<'a>(&mut self, command: &'a str) -> Result<(), MatchError<'a>> {
        let runner = self.tree.traverse(command.trim_start())?;
        runner(&mut self.context);
        Ok(())
    }
}

pub struct CommandNode<C> {
    matcher: Box<dyn Matcher>,
    // TODO: make sure two children cannot overlap: return an error/panic
    children: Vec<CommandNode<C>>,
    executes: Option<Box<dyn Fn(&mut C)>>,
}

impl<C> CommandNode<C> {
    pub fn root() -> Self {
        Self::new(matchers::Wildcard)
    }

    pub fn new(matcher: impl Matcher + 'static) -> Self {
        Self {
            matcher: Box::new(matcher),
            children: Vec::new(),
            executes: None,
        }
    }

    pub(crate) fn traverse<'a>(
        &self,
        mut command: &'a str,
    ) -> Result<&dyn Fn(&mut C), MatchError<'a>> {
        let advance = self.matcher.apply(&command)?;
        command = &command[advance..].trim_start();
        println!("matcher {:?} consumed {} characters", self.matcher, advance);

        // Check for empty rest before excessively iterating over each child node
        if command.is_empty() {
            return self
                .executes
                .as_ref()
                .ok_or(MatchError::EndOfInput)
                .map(|r| &**r);
        }

        for child in &self.children {
            if let Ok(run) = child.traverse(command) {
                return Ok(run);
            }
        }

        Err(MatchError::InvalidInput(command))
    }

    pub fn then(mut self, node: Self) -> Self {
        self.children.push(node);
        self
    }

    pub fn runs(mut self, f: impl Fn(&mut C) + 'static) -> Self {
        self.executes = Some(Box::new(f));
        self
    }
}

pub fn literal<C>(lit: impl ToString) -> CommandNode<C> {
    CommandNode::new(matchers::Literal(lit.to_string()))
}

#[cfg(test)]
mod test {
    #[test]
    fn test_traversal() {
        use super::*;

        let mut dispatcher = Dispatcher::new(());
        dispatcher.register(
            literal("foo")
                .then(literal("bar").runs(|_| println!("foo bar")))
                .then(literal("baz").runs(|_| println!("foo baz"))),
        );
        dispatcher.register(literal("qux").runs(|_| println!("qux")));

        dispatcher.dispatch("foo bar").unwrap();
        dispatcher.dispatch("foo baz").unwrap();
        dispatcher.dispatch("foo").unwrap_err();
        dispatcher.dispatch("qux").unwrap();
    }
}
