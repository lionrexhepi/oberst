oberst_proc::define_command! { foo: Context {
    #[usage = "with <arg>"]
    fn bar(ctx: &Context, arg: u32) -> Result<(), CommandError<'static>> {
        println!("bar with {}", arg);
        Ok(())
    }
}
}

use crate::*;
struct Context;
