//! `!bug` - A prioritized bug list for users.
//!
//! # Usage
//!
//! * `!bug [show, print, display]` - Print the bug list.
//! * `!bug [show, print, display] <BUG_NUMBER>` - Print the details for a specific bug.
//! * `!bug [report] <BUG_NAME> <BUG_SUMMARY> <BUG_DETAILS>` - Add a bug to the list.
//! * `!bug (remove, rm, delete) <BUG_NUMBER>` - Remove a bug from the list.
//! * `!bug +1 <BUG_NUMBER>` - Report that you've also encoutered this bug.

use anyhow::Result;
use mongodb::{bson::doc, Database};
use serde::{Deserialize, Serialize};
use serenity::model::prelude::{Message, UserId};
use std::{collections::HashMap, fmt};
use tracing::{debug, info};

static COLLECTION_NAME: &str = "global_bugs";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BugStatus {
    Open,
    Closed,
}

impl fmt::Display for BugStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BugStatus::Open => write!(f, "Open"),
            BugStatus::Closed => write!(f, "Closed"),
        }
    }
}

impl Default for BugStatus {
    fn default() -> Self {
        BugStatus::Open
    }
}

/// A global list of bugs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BugList {
    /// The bugs in the global list. The key is the item key, and the value is the
    /// item state.
    items: HashMap<String, BugItem>,
}

impl BugList {
    fn new() -> Self {
        BugList {
            items: Default::default(),
        }
    }
}

/// A single bug item in a user's bug list.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BugItem {
    // Hopefully our code isn't so bad that we'll need more than this.
    pub number: u32,
    pub priority: u32,
    pub status: BugStatus,
    pub labels: Vec<String>,
    pub name: String,
    pub summary: String,
    pub details: String,
    pub reporter: UserId,
    pub plus_ones: Vec<UserId>,
}

/// Loads the user's bug list state from the database and then process the
/// user's message.
pub async fn message(db: &Database, msg: &Message) -> Result<String> {
    let user_id = msg.author.id;

    // Get the collection of user bug lists and find the document for the user that
    // sent the message.
    let collection = db.collection(COLLECTION_NAME);
    let query = doc! { "user_id": user_id.to_string() };

    // Attempt to load the user's bug list state from the database.
    let doc = collection.find_one(query.clone(), None).await?;
    debug!("Loaded global bug list: {doc:#?}");

    // If this is the first time the user is using the `!bug` command we need to
    // insert a new document for the user.
    let mut user_list = match doc {
        Some(doc) => doc,

        None => {
            info!("First time usage of `!bug`, inserting empty list");

            let new = BugList::new();
            collection.insert_one(new.clone(), None).await?;
            new
        }
    };

    // Handle the message, updating `bug_state` and getting the response message.
    let response = handle_message(&mut user_list, msg)?;

    // Write the updated bug state to the database.
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

    Ok(response)
}

/// Performs the core logic for handling a `!bug` command.
///
/// Updates the state of `bug_list` to reflect the new list state, and returns
/// the message that should be sent back to the channel where the command was
/// given.
pub fn handle_message(bug_list: &mut BugList, msg: &Message) -> Result<String> {
    #[derive(Debug, Clone, Copy)]
    enum BugCommand {
        Report,
        Remove,
        PlusOne,
        Print,
        PrintAll,
        Help,
    }

    // Get the user's bug list, creating a new empty one if the user doesn't already
    // have a bug list.
    let user_id = msg.author.id;

    // Strip "!bug" off the front to get the body of the command.
    let body = msg.content.strip_prefix("!bug").unwrap().trim();

    // Split off the first word of the body and see if it's a known command,
    // converting the rest of the body into the new bug item key.
    let (command, key) = match body.split_once(char::is_whitespace) {
        Some(("" | "show" | "print" | "display", key)) => (BugCommand::Print, key),
        Some(("report" | "add", key)) => (BugCommand::Report, key),
        Some(("remove" | "rm" | "delete", key)) => (BugCommand::Remove, key),
        Some(("+1", key)) => (BugCommand::PlusOne, key),

        // If there's no body, print the bug list.
        None if body.is_empty() => (BugCommand::PrintAll, body),
        None => (BugCommand::Report, body),

        // If the user didn't specify a command (e.g. "!bug foo bar baz") then assume
        // they want to see some help
        _ => (BugCommand::Help, body),
    };

    debug!(
        "Parsed !bug command {:?} to command {command:?} and key {key:?}",
        msg.content,
    );

    // Handle the selected command.
    match command {
        BugCommand::Report => {
            todo!("Implement bug reporting");
            // let mut response = String::new();
            // Ok(response)
        }

        BugCommand::Remove => {
            match bug_list.items.remove(key) {
                Some(item) => {
                    info!("User {user_id} permanently deleted bug #{key:?} {}", item.name);
                    Ok(format!("Removed bug #{key:?} from the list"))
                }
                None => Ok(format!("Bug #{key:?} not found in your list")),
            }
        }

        BugCommand::Print => {
            if let Some(entry) = bug_list.items.get_mut(key) {
                // TODO how does adding metadata to the `info` macro work?
                // info!(key = key, user_id = user_id, "Printing bug info");
                let mut response = format!("#{key} {}\n", entry.name);
                response.push_str(&format!("{}:\n", entry.summary));
                response.push_str(&format!("{}:\n", entry.details));
                response.push_str(&format!("Priority: {}\n", entry.priority));
                response.push_str(&format!("Status: {}\n", entry.status));
                response.push_str(&format!("Labels: {}\n", entry.labels.join(", ")));
                response.push_str(&format!("Reporter: {}\n", entry.reporter));
                response.push_str(&format!("Plus Ones: {}\n", entry.plus_ones.len()));

                Ok(response)
            } else {
                Ok(format!(
                    "I couldn't find a bug with the number {key} in the global list."
                ))
            }
        }
        BugCommand::PlusOne => {
            if let Some(entry) = bug_list.items.get_mut(key) {
                entry.plus_ones.push(user_id);
                Ok(format!("I'm sorry to hear that you're also experiencing this issue.\nAt least you've got {} other(s) for company.", entry.plus_ones.len()))
            } else {
                Ok(format!(
                    "I couldn't find a bug with the number {key} in the global list."
                ))
            }
        }
        BugCommand::PrintAll => {
            todo!("Create a table of all bugs and return it");
            
            // let mut response = String::new();
            // Ok(response)
        }
        BugCommand::Help => {
            todo!("surely I'm reinventing the wheel here")
        }
    }
}
