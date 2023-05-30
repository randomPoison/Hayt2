use mongodb::Database;
use serenity::prelude::*;
use std::sync::Arc;

pub mod todo;

pub struct Bot {
    db: Database,
}

impl TypeMapKey for Bot {
    type Value = Arc<Bot>;
}

impl Bot {
    pub fn new(db: Database) -> Self {
        Bot { db }
    }
}
