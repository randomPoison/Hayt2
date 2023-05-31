//! `!todo` - A prioritized TODO list for users.
//!
//! # Usage
//!
//! * `!todo [show, print, display]` - Print your TODO list.
//! * `!todo [add] <ITEM_KEY>` - Add an item to the list.
//! * `!todo (remove, rm, delete) <ITEM_KEY>` - Remove an item from the list.
//! * `!todo (done, finish, finished, x, X) <ITEM_KEY>` - Mark an item done.
//!
//! # Item Prioritization
//!
//! Each item is given a priority value in order to bubble higher priority items
//! to the top of your list. Each time you add an item to your list it increases
//! the priority by 1. By default the list is printed in priority order.

use crate::{serenity, Error};
use anyhow::Result;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use tracing::{debug, error, info};

static COLLECTION_NAME: &str = "user_todos";

/// A TODO list for a single user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodoList {
    user_id: serenity::UserId,

    /// The items in the user's list. The key is the item key, and the value is the
    /// item state.
    items: HashMap<String, TodoItem>,
}

impl TodoList {
    fn new(user_id: serenity::UserId) -> Self {
        TodoList {
            user_id,
            items: Default::default(),
        }
    }
}

/// A single TODO item in a user's TODO list.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub priority: u32,
    pub done: bool,
}

/*
/// Loads the user's TODO list state from the database and then process the
/// user's message.
#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    let user_id = msg.author.id;

    // Get the bot state from global storage.
    let data = ctx.data.read().await;
    let bot = data
        .get::<Handler>()
        .expect("Expected CommandCounter in TypeMap.");

    // Get the collection of user TODO lists and find the document for the user that
    // sent the message.
    let collection = bot.db.collection(COLLECTION_NAME);
    let query = doc! { "user_id": user_id.to_string() };

    // Attempt to load the user's TODO list state from the database.
    let doc = collection.find_one(query.clone(), None).await?;
    debug!("Loaded TODO list for user {user_id}: {doc:#?}");

    // If this is the first time the user is using the `!todo` command we need to
    // insert a new document for the user.
    let mut user_list = match doc {
        Some(doc) => doc,

        None => {
            info!("First time usage of `!todo` for user {user_id}, inserting empty list");

            let new = TodoList::new(user_id);
            collection.insert_one(new.clone(), None).await?;
            new
        }
    };

    // Handle the message, updating `todo_state` and getting the response message.
    let response = handle_message(&mut user_list, &msg)?;

    // Write the updated TODO state to the database.
    collection
        .update_one(
            query,
            doc! {
                "$set": {
                    "items": bson::to_bson(&user_list.items).unwrap(),
                },
            },
            None,
        )
        .await?;

    // Send the response to the channel where the command was sent.
    if let Err(e) = msg.channel_id.say(&ctx.http, response).await {
        error!("Error sending message: {:?}", e);
    }

    Ok(())
}
*/

/// Performs the core logic for handling a `!todo` command.
///
/// Updates the state of `todo_list` to reflect the new list state, and returns
/// the message that should be sent back to the channel where the command was
/// given.
pub fn handle_message(todo_list: &mut TodoList, msg: &serenity::Message) -> Result<String> {
    #[derive(Debug, Clone, Copy)]
    enum TodoCommand {
        Add,
        Remove,
        Finish,
        Print,
    }

    // Get the user's TODO list, creating a new empty one if the user doesn't already
    // have a TODO list.
    let user_id = msg.author.id;

    // Strip "!todo" off the front to get the body of the command.
    let body = msg.content.strip_prefix("!todo").unwrap().trim();

    // Split off the first word of the body and see if it's a known command,
    // converting the rest of the body into the new TODO item key.
    let (command, key) = match body.split_once(char::is_whitespace) {
        Some(("" | "show" | "print" | "display", key)) => (TodoCommand::Print, key),
        Some(("add", key)) => (TodoCommand::Add, key),
        Some(("done" | "finish" | "finished" | "x" | "X", key)) => (TodoCommand::Finish, key),
        Some(("remove" | "rm" | "delete", key)) => (TodoCommand::Remove, key),

        // If there's no body, print the TODO list.
        None if body.is_empty() => (TodoCommand::Print, ""),
        None => (TodoCommand::Add, body),

        // If the user didn't specify a command (e.g. "!todo foo bar baz") then assume
        // they just want to add to their TODO list.
        _ => (TodoCommand::Add, body),
    };

    debug!(
        "Parsed !todo command {:?} to command {command:?} and key {key:?}",
        msg.content,
    );

    // Handle the selected command.
    match command {
        TodoCommand::Add => {
            let item = todo_list.items.entry(key.into()).or_default();
            item.priority += 1;

            info!(
                "Updated TODO item {key:?} for user {user_id}, priority: {}",
                item.priority,
            );

            let response = match item.priority {
                1 => format!("Added item {key:?} to your list"),
                _ => format!("Updated item {key:?}, priority is {}", item.priority),
            };

            Ok(response)
        }

        TodoCommand::Remove => {
            let _old = todo_list.items.remove(key);

            info!("Removed TODO item {key:?} for user {user_id}");

            Ok(format!("Removed {key:?} from your list"))
        }

        TodoCommand::Finish => {
            let item = todo_list.items.entry(key.into()).or_default();
            item.done = true;

            info!("Finished TODO item {key:?} for user {user_id}");

            Ok(format!("Marked {key:?} as done"))
        }

        TodoCommand::Print => {
            info!("Printing TODO list for user {user_id}");

            let user_name = &msg.author.name;
            let mut response = format!("TODO list for {user_name}:\n");

            // Get a list of the TODO list keys and sort it by item priority so that we
            // can display the list in priority order.
            let mut sorted_keys = todo_list
                .items
                .iter()
                .map(|(key, val)| (val.priority, key))
                .collect::<Vec<_>>();
            sorted_keys.sort_by_key(|(priority, _)| *priority);

            // Build a string that displays the TODO list.
            //
            // NOTE: We iterate over the sorted keys in reverse order because
            // `sort_by_key` sorts in ascending order and we want to print the list in
            // descending order.
            for &(_, key) in sorted_keys.iter().rev() {
                let item = &todo_list.items[key];
                let check_mark = if item.done { 'X' } else { ' ' };
                writeln!(&mut response, "> [{check_mark}] {key}").unwrap();
            }

            Ok(response)
        }
    }
}
