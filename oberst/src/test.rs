oberst_proc::define_command! { foo: Context {
    fn bar(ctx: &Context, arg: u32) -> Result<(), CommandError<'static>> {
        println!("bar: {}", arg);
        Ok(())
    }
}
}

use crate::*;
struct Context;
