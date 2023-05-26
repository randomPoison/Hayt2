use crate::Bot;
use serenity::{model::prelude::Message, prelude::Context};
use std::collections::HashMap;
use tracing::info;

pub type TodoList = HashMap<String, TodoItem>;

#[derive(Debug, Default, Clone)]
pub struct TodoItem {
    pub priority: u32,
    pub done: bool,
}

#[derive(Debug, Clone, Copy)]
enum TodoCommand {
    Add,
    Remove,
    Finish,
    Print,
}

pub async fn handle_message(bot: &Bot, _ctx: Context, msg: Message) {
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
        Some(("add", key)) => (TodoCommand::Add, key),
        Some(("done" | "finish" | "finished" | "x" | "X", key)) => (TodoCommand::Finish, key),
        Some(("remove" | "delete", key)) => (TodoCommand::Remove, key),

        // Print the TODO list if the body is empty.
        Some(("", _)) => (TodoCommand::Print, ""),

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
