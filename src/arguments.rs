use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub struct CommandArgs {
    internal: HashMap<String, Box<dyn Any>>,
}

impl CommandArgs {
    pub fn new() -> Self {
        Self {
            internal: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, key: &str, value: Box<dyn Any>) {
        self.internal.insert(key.to_string(), value);
    }

    pub fn get<T: 'static>(&self, key: &str) -> Option<&T> {
        self.internal.get(key).and_then(|value| {
            if value.type_id() == TypeId::of::<T>() {
                Some(value.downcast_ref::<T>().unwrap())
            } else {
                None
            }
        })
    }
}
