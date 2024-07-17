use crate::*;

define_command! {test(i32) {
    fn test(context: &i32, ) -> CommandResult {
        Ok(0)
    }
}}
