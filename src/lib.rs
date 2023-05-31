use mongodb::Database;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info, trace};

pub mod bug;
pub mod todo;

pub struct Bot {
    db: Database,
}

impl Bot {
    pub fn new(db: Database) -> Self {
        Bot { db }
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        let user_id = msg.author.id;
        trace!("Received message from user {user_id}: {:?}", msg.content);

        // Handle the message based on the command it starts with.
        let response: anyhow::Result<Option<String>> = if msg.content == "!hello" {
            Ok(Some("world!".into()))
        } else if msg.content.starts_with("!todo") {
            todo::message(&self.db, &msg).await.map(Some)
        } else if msg.content.starts_with("!bug") {
            bug::message(&self.db, &msg).await.map(Some)
        } else if msg.content.starts_with("!help") {
            Ok(Some(
                "(chuckles) Oh, my... There's no.. help. for you, I'm afraid.".to_owned(),
            ))
        } else {
            Ok(None)
        };

        // Handle any error that occurred while handling the message.
        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                error!(
                    "Error occurred responding to message {:?} from user {user_id}: {e:?}",
                    msg.content,
                );

                let message = format!(
                    "Error occurred:\n\
                    ```\n\
                    {e}\n\
                    ```",
                );

                if let Err(e) = msg.channel_id.say(&ctx.http, message).await {
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
