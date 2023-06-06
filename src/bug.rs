//! `!bug` - A prioritized bug list for users.
//!
//! # Usage
//!
//! * `!bug [show, print, display]` - Print the bug list.
//! * `!bug [show, print, display] <BUG_NUMBER>` - Print the details for a specific bug.
//! * `!bug [report] <BUG_NAME> <BUG_SUMMARY> <BUG_DETAILS>` - Add a bug to the list.
//! * `!bug (remove, rm, delete) <BUG_NUMBER>` - Remove a bug from the list.
//! * `!bug +1 <BUG_NUMBER>` - Report that you've also encoutered this bug.

use anyhow::{Context, Ok, Result};
use mongodb::{bson::doc, Database};
use pest::Parser;
use serde::{Deserialize, Serialize};
use serenity::model::prelude::{Message, UserId};
use std::{collections::HashMap, fmt};
use tracing::{debug, info};

static COLLECTION_NAME: &str = "global_bugs";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    items: HashMap<u32, BugItem>,
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
pub async fn message(db: &Database, msg: &Message) -> anyhow::Result<String> {
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
pub fn handle_message(bug_list: &mut BugList, msg: &Message) -> anyhow::Result<String> {
    #[derive(Debug, Clone, Copy)]
    enum BugCommand {
        Report,
        // TODO(id-generation) Don't activate this command until we have a reliable way to generate
        // bug numbers other than just checking the current number of bugs.
        // Remove,
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
    let (command, rest) = match body.split_once(char::is_whitespace) {
        Some(("" | "show" | "print" | "display", rest)) => (BugCommand::Print, rest),
        Some(("report" | "add", rest)) => (BugCommand::Report, rest),
        // TODO(id-generation)
        // Some(("remove" | "rm" | "delete", rest)) => (BugCommand::Remove, rest),
        Some(("+1", rest)) => (BugCommand::PlusOne, rest),

        // If there's no body, print the bug list.
        None if body.is_empty() => (BugCommand::PrintAll, body),
        None => (BugCommand::Report, body),

        // If the user didn't specify a command (e.g. "!bug foo bar baz") then assume
        // they want to see some help
        _ => (BugCommand::Help, body),
    };

    debug!(
        "Parsed !bug command {:?} to command {command:?} and key {rest:?}",
        msg.content,
    );

    // Handle the selected command.
    match command {
        // Add the new bug to the database if the information passed by the user is valid. Otherwise, respond with an error message.
        BugCommand::Report => {
            let mut parsed_report = BugReportParser::parse(Rule::bug_report, rest.trim())
                .context("couldn't parse a user-submitted bug report")?;

            let name = parsed_report
                .next()
                .expect("parser says this exists")
                .as_str()
                .trim_matches('"');
            let summary = parsed_report
                .next()
                .expect("parser says this exists")
                .as_str()
                .trim_matches('"');
            let detail = parsed_report
                .next()
                .expect("parser says this exists")
                .as_str()
                .trim_matches('"');

            // TODO(id-generation) this approach is sound only so long as no bugs are ever removed from the list.
            let new_bug_number = bug_list.items.len() as u32 + 1;
            let new_bug = BugItem {
                number: new_bug_number,
                name: name.to_string(),
                summary: summary.to_string(),
                details: detail.to_string(),
                reporter: user_id,
                ..Default::default()
            };

            bug_list.items.insert(new_bug.number, new_bug);

            Ok(format!(
                "Added bug #{new_bug_number} \"{name}\" to the list",
            ))
        }
        // TODO(id-generation) Don't activate this command until we have a reliable way to generate
        // bug numbers other than just checking the current number of bugs.
        // BugCommand::Remove => match bug_list.items.remove(rest) {
        //     Some(item) => {
        //         info!(
        //             "User {user_id} permanently deleted bug #{rest} {}",
        //             item.name
        //         );
        //         Ok(format!("Removed bug #{rest} from the list"))
        //     }
        //     None => Ok(format!("Bug #{rest} not found in your list")),
        // },
        BugCommand::Print => {
            let bug_number = normalize_bug_number(rest)?;
            if let Some(entry) = bug_list.items.get_mut(&bug_number) {
                // TODO how does adding metadata to the `info` macro work?
                // info!(key = key, user_id = user_id, "Printing bug info");
                let mut response = format!("#{rest} {}\n", entry.name);
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
                    "I couldn't find a bug with the number {rest} in the global list."
                ))
            }
        }
        BugCommand::PlusOne => {
            let bug_number = normalize_bug_number(rest)?;
            if let Some(entry) = bug_list.items.get_mut(&bug_number) {
                entry.plus_ones.push(user_id);
                Ok(format!("I'm sorry to hear that you're also experiencing this issue.\nAt least you've got {} other(s) for company.", entry.plus_ones.len() - 1))
            } else {
                Ok(format!(
                    "I couldn't find a bug with the number {rest} in the global list."
                ))
            }
        }
        BugCommand::PrintAll => {
            info!("Listing all unclosed bugs");
            let mut response = String::new();

            let unclosed_bugs = bug_list
                .items
                .values()
                .filter(|&bug| bug.status != BugStatus::Closed);

            for bug in unclosed_bugs {
                let BugItem {
                    name,
                    summary,
                    labels,
                    plus_ones,
                    number,
                    ..
                } = bug;
                response.push_str(&format!(
                    "#{number} {name}\t{summary}\t({} +1s)\t[{}]",
                    plus_ones.len(),
                    labels.join(", ")
                ));
            }

            Ok(response)
        }
        BugCommand::Help => {
            todo!("surely I'm reinventing the wheel here")
        }
    }
}

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
bug_report = {
    string_literal ~ WS ~
    string_literal ~ WS ~
    string_literal
}
// Either a triple-quoted string, quoted string, or a single "word"
string_literal = @{ triple_quoted_string | double_quoted_string | (!WS ~ ANY)* }
double_quoted_string = { DOUBLE_QUOTE ~ (!DOUBLE_QUOTE ~ ANY)* ~ DOUBLE_QUOTE }
triple_quoted_string = { TRIPLE_QUOTE ~ (!TRIPLE_QUOTE ~ ANY)* ~ TRIPLE_QUOTE }
WS = _{ " " }
TRIPLE_QUOTE = { "\"\"\"" }
DOUBLE_QUOTE = { "\"" }
"#]
struct BugReportParser;

/// User-supplied bug numbers can be formatted in a variety of ways. This function
/// normalizes the bug number to a consistent format, or returns an error if the
/// bug number can't be normalized.
fn normalize_bug_number(key: &str) -> Result<u32> {
    key.trim().parse().context(format!(
        "couldn't parse bug number from user input \"{key}\""
    ))
}
