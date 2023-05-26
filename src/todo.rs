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
use std::fmt::Write;
use tracing::{error, info, debug};

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

pub async fn handle_message(bot: &Bot, ctx: Context, msg: Message) {
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

        // If there's no body, print the TODO list.
        None => (TodoCommand::Print, ""),

        // If the user didn't specify a command (e.g. "!todo foo bar baz") then assume
        // they just want to add to their TODO list.
        _ => (TodoCommand::Add, body),
    };

    debug!("Parsed !todo command {:?} to command {command:?} and key {key:?}", msg.content);

    // Handle the selected command.
    match command {
        TodoCommand::Add => {
            let item = todo_list.entry(key.into()).or_default();
            item.priority += 1;

            info!(
                "Updated TODO item {key:?} for user {user_id}, priority: {}",
                item.priority,
            );

            let response = match item.priority {
                1 => format!("Added item {key:?} to your list"),
                _ => format!("Updated item {key:?}, priority is {}", item.priority),
            };

            if let Err(e) = msg.channel_id.say(&ctx.http, response).await {
                error!("Error sending message: {:?}", e);
            }
        }

        TodoCommand::Remove => {
            let _old = todo_list.remove(key);

            info!("Removed TODO item {key:?} for user {user_id}");

            let response = format!("Removed {key:?} from your list");
            if let Err(e) = msg.channel_id.say(&ctx.http, response).await {
                error!("Error sending message: {:?}", e);
            }
        }

        TodoCommand::Finish => {
            let item = todo_list.entry(key.into()).or_default();
            item.done = true;

            info!("Finished TODO item {key:?} for user {user_id}");

            let response = format!("Marked {key:?} as done");
            if let Err(e) = msg.channel_id.say(&ctx.http, response).await {
                error!("Error sending message: {:?}", e);
            }
        }

        TodoCommand::Print => {
            info!("Printing TODO list for user {user_id}");

            let user_name = &msg.author.name;
            let mut response = format!("TODO list for {user_name}:\n");

            // TODO: Sort by priority.
            for (key, item) in todo_list {
                let check_mark = if item.done { 'X' } else { ' ' };
                writeln!(&mut response, "> [{check_mark}] {key}").unwrap();
            }

            if let Err(e) = msg.channel_id.say(&ctx.http, response).await {
                error!("Error sending message: {:?}", e);
            }
        }
    }
}
