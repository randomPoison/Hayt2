//! `!todo` - A prioritized TODO list for users.
//!
//! # Usage
//!
//! * `!todo [show, print, display]` - Print your TODO list.
//! * `!todo [add] <ITEM_KEY>` - Add an item to the list.
//! * `!todo (remove, delete) <ITEM_KEY>` - Remove an item from the list.
//! * `!todo (done, finish, finished, x, X) <ITEM_KEY>` - Mark an item done.
//!
//! # Item Prioritization
//!
//! Each item is given a priority value in order to bubble higher priority items
//! to the top of your list. Each time you add an item to your list it increases
//! the priority by 1. By default the list is printed in priority order.

use crate::Bot;
use serenity::{model::prelude::Message, prelude::Context};
use std::collections::HashMap;
use tracing::info;

/// A TODO list for a single user.
///
/// The key is the item key, and the value is a [TodoItem] containing the saved
/// TODO item state, i.e. the priority.
pub type TodoList = HashMap<String, TodoItem>;

/// A single TODO item in a user's TODO list.
#[derive(Debug, Default, Clone)]
pub struct TodoItem {
    pub priority: u32,
    pub done: bool,
}

pub async fn handle_message(bot: &Bot, _ctx: Context, msg: Message) {
    #[derive(Debug, Clone, Copy)]
    enum TodoCommand {
        Add,
        Remove,
        Finish,
        Print,
    }

    let user_id = msg.author.id;

    // Lock the TODO list state and get the user's TODO list, creating a new empty
    // one if the user doesn't already have a TODO list.
    let mut todo_state = bot.todo_state.lock().await;
    let todo_list = todo_state.entry(user_id).or_default();

    // Strip "!todo" off the front to get the body of the command.
    let body = msg.content.strip_prefix("!todo").unwrap().trim();

    // Split off the first word of the body and see if it's a known command,
    // converting the rest of the body into the new TODO item key.
    let (command, key) = match body.split_once(char::is_whitespace) {
        Some(("" | "show" | "print" | "display", key)) => (TodoCommand::Print, key),
        Some(("add", key)) => (TodoCommand::Add, key),
        Some(("done" | "finish" | "finished" | "x" | "X", key)) => (TodoCommand::Finish, key),
        Some(("remove" | "delete", key)) => (TodoCommand::Remove, key),

        // If the user didn't specify a command (e.g. "!todo foo bar baz") then assume
        // they just want to add to their TODO list.
        _ => (TodoCommand::Add, body),
    };

    // Handle the selected command.
    match command {
        TodoCommand::Add => {
            let item = todo_list.entry(key.into()).or_default();
            item.priority += 1;

            // TODO: Respond indicating what the new priority is.
            info!(
                "Updated TODO item {key:?} for user {user_id}, priority: {}",
                item.priority,
            );
        }

        TodoCommand::Remove => {
            let _old = todo_list.remove(key);

            // TODO: Respond indicating that the item was removed.
            info!("Removed TODO item {key:?} for user {user_id}");
        }

        TodoCommand::Finish => {
            let item = todo_list.entry(key.into()).or_default();
            item.done = true;

            // TODO: Respond indicating that item was completed.
            info!("Finished TODO item {key:?} for user {user_id}");
        }

        TodoCommand::Print => {
            // TODO: Print the list.
        }
    }
}
