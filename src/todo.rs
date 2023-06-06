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

use crate::{serenity, Context, Error};
use anyhow::{Context as _, Result};
use mongodb::bson::doc;
use poise::serenity_prelude::{CacheHttp, User};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use tracing::{debug, error, info};

#[poise::command(
    prefix_command,
    slash_command,
    subcommands("show", "add", "remove", "done")
)]
pub async fn todo(
    ctx: Context<'_>,
    key: Option<String>,
    category: Option<String>,
) -> Result<(), Error> {
    match key {
        Some(key) => run_command(ctx, TodoCommand::Add { key, category }).await,
        None => run_command(ctx, TodoCommand::Print { category }).await,
    }
}

#[poise::command(prefix_command, slash_command)]
pub async fn show(ctx: Context<'_>, category: Option<String>) -> Result<(), Error> {
    run_command(ctx, TodoCommand::Print { category }).await
}

#[poise::command(prefix_command, slash_command)]
pub async fn add(ctx: Context<'_>, key: String, category: Option<String>) -> Result<(), Error> {
    run_command(ctx, TodoCommand::Add { key, category }).await
}

#[poise::command(prefix_command, slash_command)]
pub async fn remove(ctx: Context<'_>, key: String) -> Result<(), Error> {
    run_command(ctx, TodoCommand::Remove(key)).await
}

#[poise::command(prefix_command, slash_command)]
pub async fn done(ctx: Context<'_>, key: String) -> Result<(), Error> {
    run_command(ctx, TodoCommand::Finish(key)).await
}

/// Loads the user's TODO list state from the database and then process the
/// command.
async fn run_command(ctx: Context<'_>, command: TodoCommand) -> Result<()> {
    let user_id = ctx.author().id;

    // Get the collection of user TODO lists and find the document for the user that
    // sent the message.
    let collection = ctx.data().db.collection("user_todos");
    let query = doc! { "user_id": user_id.to_string() };

    // Attempt to load the user's TODO list state from the database.
    let doc = collection
        .find_one(query.clone(), None)
        .await
        .with_context(|| format!("Failed to get TODO list for user {user_id}"))?;
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
    let response = handle_command(command, &mut user_list, ctx.author());

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
        .await
        .with_context(|| format!("Failed to update TODO items for user {user_id}"))?;

    // Send the response to the channel where the command was sent.
    if let Err(e) = ctx.channel_id().say(ctx.http(), response).await {
        error!("Error sending message: {:?}", e);
    }

    Ok(())
}

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
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
enum TodoCommand {
    Print {
        category: Option<String>,
    },

    Add {
        key: String,
        category: Option<String>,
    },

    Remove(String),
    Finish(String),
}

/// Performs the core logic for handling a `!todo` command.
///
/// Updates the state of `todo_list` to reflect the new list state, and returns
/// the message that should be sent back to the channel where the command was
/// given.
fn handle_command(command: TodoCommand, todo_list: &mut TodoList, author: &User) -> String {
    let user_id = author.id;

    // Handle the selected command.
    match command {
        TodoCommand::Add { key, category } => {
            let item = todo_list.items.entry(key.clone()).or_default();
            item.priority += 1;

            // Update the item's category if one was specified.
            if category.is_some() {
                item.category = category;
            }

            let key_display = match &item.category {
                Some(category) => format!("[{category}] {key:?}"),
                None => format!("{key:?}"),
            };

            info!(
                "Updated TODO item {key_display} for user {user_id}, priority: {}",
                item.priority,
            );

            let response = match item.priority {
                1 => format!("Added item {key_display} to your list"),
                _ => format!("Updated item {key_display}, priority is {}", item.priority),
            };

            response
        }

        TodoCommand::Remove(key) => {
            let _old = todo_list.items.remove(&key);

            info!("Removed TODO item {key:?} for user {user_id}");

            format!("Removed {key:?} from your list")
        }

        TodoCommand::Finish(key) => {
            let item = todo_list.items.entry(key.clone()).or_default();
            item.done = true;

            info!("Finished TODO item {key:?} for user {user_id}");

            format!("Marked {key:?} as done")
        }

        TodoCommand::Print { category } => {
            info!("Printing TODO list for user {user_id}");

            let user_name = &author.name;
            let mut response = match &category {
                Some(category) => format!("TODO list for {user_name} in category [{category}]:\n"),
                None => format!("TODO list for {user_name}:\n"),
            };

            // Get a list of the TODO list keys and sort it by item priority so that we
            // can display the list in priority order.
            let mut sorted_keys = todo_list
                .items
                .iter()
                .filter(|(_, val)| category.is_none() || val.category == category)
                .map(|(key, val)| (val.priority, key))
                .collect::<Vec<_>>();
            sorted_keys.sort_by_key(|(priority, _)| *priority);

            // Determine how wide the priority output needs to be displayed by finding the
            // highest priority and calculating how many digits it will be.
            let max_priority = todo_list
                .items
                .values()
                .map(|item| item.priority)
                .max()
                .unwrap_or_default();
            let priority_width = f32::log10((max_priority + 1) as f32).ceil() as usize;

            // Build a string that displays the TODO list.
            //
            // NOTE: We iterate over the sorted keys in reverse order because
            // `sort_by_key` sorts in ascending order and we want to print the list in
            // descending order.
            response.push_str("```\n");
            for &(_, key) in sorted_keys.iter().rev() {
                let item = &todo_list.items[key];
                let check_mark = if item.done { 'X' } else { ' ' };
                let priority = item.priority;

                let category_str = if category.is_some() || item.category.is_none() {
                    "".into()
                } else {
                    format!(" [{}]", item.category.as_ref().unwrap())
                };

                writeln!(
                    &mut response,
                    "({priority: >priority_width$}) [{check_mark}]{category_str} {key}"
                )
                .unwrap();
            }
            response.push_str("```\n");

            response
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::todo::{self, TodoCommand, TodoList};
    use poise::serenity_prelude::model::user::User;
    use pretty_assertions::assert_eq;

    static USER_NAME: &str = "randomPoison";

    /// Builds a [Message] from the given `text`.
    fn send_command(command: TodoCommand, state: &mut TodoList) -> String {
        let mut user = User::default();
        user.name = USER_NAME.into();

        todo::handle_command(command, state, &user)
    }

    // Adds an item and verifies that the response is correct.
    fn add_item(state: &mut TodoList, key: impl Into<String>, priority: u32) {
        let key = key.into();
        let response = send_command(
            TodoCommand::Add {
                key: key.clone(),
                category: None,
            },
            state,
        );

        let expected = match priority {
            1 => format!("Added item {key:?} to your list"),
            _ => format!("Updated item {key:?}, priority is {priority}"),
        };
        assert_eq!(expected, response);
    }

    // Adds an item and verifies that the response is correct.
    fn add_with_category(
        state: &mut TodoList,
        key: impl Into<String>,
        category: impl Into<String>,
        priority: u32,
    ) {
        let key = key.into();
        let category = category.into();

        let response = send_command(
            TodoCommand::Add {
                key: key.clone(),
                category: Some(category.clone()),
            },
            state,
        );

        let expected = match priority {
            1 => format!("Added item [{category}] {key:?} to your list"),
            _ => format!("Updated item [{category}] {key:?}, priority is {priority}"),
        };
        assert_eq!(expected, response);
    }

    /// Tests that an item can be added from the list, displayed, and then removed.
    #[test]
    fn add_display_remove() {
        let mut state = TodoList::default();

        // Add an item with the key "foo" to the list.
        add_item(&mut state, "foo", 1);

        // Verify that the item can be displayed in the TODO list.
        let response = send_command(TodoCommand::Print { category: None }, &mut state);
        assert_eq!(
            format!(
                "TODO list for {USER_NAME}:\n\
                ```\n\
                (1) [ ] foo\n\
                ```\n"
            ),
            response,
        );

        // Remove the item from the list.
        let response = send_command(TodoCommand::Remove("foo".into()), &mut state);
        assert_eq!(r#"Removed "foo" from your list"#, response);

        // Verify that the list is now empty when printed.
        let response = send_command(TodoCommand::Print { category: None }, &mut state);
        assert_eq!(
            format!(
                "TODO list for {USER_NAME}:\n\
                ```\n\
                ```\n"
            ),
            response,
        );
    }

    // Verifies that items in the TODO list are displayed in priority order.
    #[test]
    fn priority_sort() {
        let mut state = TodoList::default();

        // Create 3 TODO items, each with different priority values.
        add_item(&mut state, "foo", 1);
        add_item(&mut state, "foo", 2);
        add_item(&mut state, "foo", 3);
        add_item(&mut state, "foo", 4);
        add_item(&mut state, "foo", 5);
        add_item(&mut state, "foo", 6);
        add_item(&mut state, "foo", 7);
        add_item(&mut state, "foo", 8);
        add_item(&mut state, "foo", 9);
        add_item(&mut state, "foo", 10);

        add_item(&mut state, "foo bar", 1);
        add_item(&mut state, "foo bar", 2);

        add_item(&mut state, "foo bar baz", 1);

        // Verify that the items are displayed in the correct order.
        let response = send_command(TodoCommand::Print { category: None }, &mut state);
        assert_eq!(
            format!(
                "TODO list for {USER_NAME}:\n\
                ```\n\
                (10) [ ] foo\n\
                ( 2) [ ] foo bar\n\
                ( 1) [ ] foo bar baz\n\
                ```\n"
            ),
            response,
        );
    }

    /// Verifies that items can be marked done.
    #[test]
    fn mark_items_done() {
        let mut state = TodoList::default();

        // Create 2 TODO items with different priority values so that they'll print
        // in a deterministic order.
        add_item(&mut state, "foo", 1);
        add_item(&mut state, "foo", 2);

        add_item(&mut state, "foo bar", 1);

        let response = send_command(TodoCommand::Finish("foo".into()), &mut state);
        assert_eq!(r#"Marked "foo" as done"#, response);

        // Verify that the items are displayed in the correct order.
        let response = send_command(TodoCommand::Print { category: None }, &mut state);
        assert_eq!(
            format!(
                "TODO list for {USER_NAME}:\n\
                ```\n\
                (2) [X] foo\n\
                (1) [ ] foo bar\n\
                ```\n"
            ),
            response,
        );
    }

    /// Verifies that a category can be set for each item and that categories are
    /// correctly handled when displaying the TODO list.
    #[test]
    fn categories() {
        let mut state = TodoList::default();

        // Create 2 TODO items with different priority values so that they'll print
        // in a deterministic order.
        add_with_category(&mut state, "foo", "Foo", 1);
        add_with_category(&mut state, "foo", "Foo", 2);
        add_item(&mut state, "foo bar", 1);

        // Verify that all items are displayed if no category is specified.
        let response = send_command(TodoCommand::Print { category: None }, &mut state);
        assert_eq!(
            format!(
                "TODO list for {USER_NAME}:\n\
                ```\n\
                (2) [ ] [Foo] foo\n\
                (1) [ ] foo bar\n\
                ```\n"
            ),
            response,
        );

        // Verify that a specific category can be displayed.
        let response = send_command(
            TodoCommand::Print {
                category: Some("Foo".into()),
            },
            &mut state,
        );
        assert_eq!(
            format!(
                "TODO list for {USER_NAME} in category [Foo]:\n\
                ```\n\
                (2) [ ] foo\n\
                ```\n"
            ),
            response,
        );

        // Verify that we can change the category of an existing item.
        add_with_category(&mut state, "foo", "Bar", 3);
        add_with_category(&mut state, "foo bar", "Foo", 2);
        let response = send_command(TodoCommand::Print { category: None }, &mut state);
        assert_eq!(
            format!(
                "TODO list for {USER_NAME}:\n\
                ```\n\
                (3) [ ] [Bar] foo\n\
                (2) [ ] [Foo] foo bar\n\
                ```\n"
            ),
            response,
        );
    }
}
