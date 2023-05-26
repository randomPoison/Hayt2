use crate::todo::TodoState;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tokio::sync::Mutex;
use tracing::{error, info};

pub mod todo;

#[derive(Default)]
pub struct Bot {
    todo_state: Mutex<TodoState>,
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        // Handle the message based on the command it starts with.
        let response: anyhow::Result<Option<String>> = if msg.content == "!hello" {
            Ok(Some("world!".into()))
        } else if msg.content.starts_with("!todo") {
            let mut todo_state = self.todo_state.lock().await;
            todo::handle_message(&mut todo_state, &msg).map(|m| Some(m.to_string()))
        } else {
            Ok(None)
        };

        // Handle any error that occurred while handling the message.
        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                error!("Error occurred: {e:?}");

                if let Err(e) = msg
                    .channel_id
                    .say(&ctx.http, format!("Error occurred: {e}"))
                    .await
                {
                    error!("Error sending message: {:?}", e);
                }

                return;
            }
        };

        // If the command resulted in a reponse, send the response back to the
        // channel where the original message was posted.
        if let Some(response) = response {
            if let Err(e) = msg.channel_id.say(&ctx.http, response).await {
                error!("Error sending message: {:?}", e);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}
