pub(crate) mod matchers;

pub use matchers::{MatchError, Matcher};

pub struct Dispatcher<C> {
    context: C,
    tree: Vec<CommandNode<C>>,
}

impl<C> Dispatcher<C> {
    pub fn new(context: C) -> Self {
        Self {
            context,
            tree: vec![],
        }
    }

    pub fn register(&mut self, node: CommandNode<C>) {
        self.tree.push(node)
    }

    pub fn dispatch<'a>(&mut self, mut command: &'a str) -> Result<(), MatchError<'a>> {
        let mut search = &self.tree;
        let mut last_error = None;
        // Condition to end the loop when the current node does not have children
        while !search.is_empty() {
            // Go over each child of the current node
            for (i, child) in search.iter().enumerate() {
                match child.matcher.apply(command) {
                    // If the child node matches, advance through the command tree
                    Ok(len) => {
                        command = &command[len - 1..];
                        search = &child.children;
                        break;
                    }
                    Err(MatchError::EndOfInput) => {
                        if let Some(run) = &child.executes {
                            // If the input ends and the current node can be executed, do that
                            run(&mut self.context)
                        } else {
                            // If it expects more arguments, return an error
                            return Err(last_error.unwrap_or(MatchError::EndOfInput));
                        }
                    }
                    Err(e) => {
                        if search.len() < i - 1 {
                            last_error = Some(e)
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
        if let Some(e) = last_error {
            // Return the last command that failed to match
            Err(e)
        } else {
            //No commands were registered
            Ok(())
        }
    }
}

pub struct CommandNode<C> {
    matcher: Box<dyn Matcher>,
    // TODO: make sure two children cannot overlap: return an error/panic
    children: Vec<CommandNode<C>>,
    executes: Option<Box<dyn Fn(&mut C)>>,
}

impl<C> CommandNode<C> {
    pub fn new(matcher: impl Matcher + 'static) -> Self {
        Self {
            matcher: Box::new(matcher),
            children: Vec::new(),
            executes: None,
        }
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

mod test {}
