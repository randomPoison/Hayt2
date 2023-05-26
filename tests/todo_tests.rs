use eval_bot::todo::{handle_message, TodoState};
use serenity::model::prelude::Message;
use serenity::utils::CustomMessage;

/// Builds a [Message] from the given `text`.
fn message(text: &str) -> Message {
    let mut builder = CustomMessage::new();
    builder.content(text.to_string());
    builder.build()
}

#[test]
fn add_and_remove() {
    let mut state = TodoState::default();

    let message = message("!todo foo");
    let response = handle_message(&mut state, &message).unwrap();
    assert_eq!(r#"Added item "foo" to your list"#, response);
}
