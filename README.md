# Oberst

A type-safe command parser & dispatcher inspired by [Brigadier](https://github.com/mojang/brigadier) and written in Rust.

## Usage

### Creating a command source

Oberst leverages Rust's procedural macros in order to generate a command's syntax from a set of ordinary functions. To use it, you first need a `CommandSource<C>`. Here, `C` can be any struct you want your commands to have access to:
```rust 
    use oberst::CommandSource;

    struct CommandContext {
        name: String
    }

    fn main() {
        let command_source = CommandSource::new(CommandContext {
            name: "Herbert".to_string()
        });
    }
```

### Defining a command

Commands are defined with the `define_command` macro : 
```rust
    use oberst::{ CommandResult, define_command}
    define_command!{hello (CommandContext) /* Specify the type of context this command needs to run */ {
        fn simple(context: &CommandContext) -> CommandResult {
            println!("Hello, {}!", &context.name);
            Ok(0) // Commands can return a "status code" that is returned to the dispatcher
        }

        // Commands can take arguments as well
        fn with_arg(context: &CommandContext, from: String) {
            println!("Hello to {} from {}", &context.name, from)
        }

        #[args = "<times> times"]
        fn custom_syntax(context: &CommandContext, times: u64) {
            for _ in 0..times {
                println!("Hello, {}!", &context.name);
            }
        }
    }}
```

Commands can accept whitespace-separated arguments of any type that implements Obersts' `Argument` trait. See the `oberst::parser` module for more info. While you can implement `Argument` for your custom types, Oberst comes with default implementation for built-in types such as integer types and `String`.

With the `args` attribute, it is possible to build a more sophisticated command syntax by allowing the command to parse both arguments and literals. However, arguments within an `args` attribute _must appear in the same order as they do in the function's signature._ 

Commands have to return either `()` or `oberst::CommandResult`. The latter supports returning any error values that implement `std::error::Error`.

### Registering a command
Commands can be registered to a source using the `register_command!` helper macro:
```rust
    fn main() {
        //...
        register_command!(command_source, hello);
        command_source.dispatch("hello \"John\""); // Prints "Hello to Herbert from John"
    }
```

## Roadmap
- [x] Command creation & dispatchment
- [x] Argument parsers for most std types
- [x] Add support for both `CommandResult` and `()` return values
- [x] Add support for custom syntax with `#[args = "..."]`
- [ ] Add support for multithreaded commands
- [x] Make `CommandSource` clonable to avoid having to pass references around