use crate::todo::TodoList;
use anyhow::anyhow;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::{async_trait, model::prelude::UserId};
use shuttle_secrets::SecretStore;
use std::collections::HashMap;
use tracing::{error, info};

mod todo;

#[derive(Default)]
pub struct Bot {
    todo_state: Mutex<HashMap<UserId, TodoList>>,
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        // Handle the message based on the command it starts with.
        let response: anyhow::Result<Option<String>> = if msg.content == "!hello" {
            Ok(Some("world!".into()))
        } else if msg.content.starts_with("!todo") {
            todo::handle_message(self, &ctx, &msg)
                .await
                .map(|m| Some(m.to_string()))
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

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&token, intents)
        .event_handler(Bot::default())
        .await
        .expect("Err creating client");

    Ok(client.into())
}
