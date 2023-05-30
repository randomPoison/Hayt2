use mongodb::Database;
use poise::serenity_prelude as serenity;
use tracing::info;
use std::sync::Arc;

pub mod todo;

type Error = serenity::Error;

pub struct Handler {
    pub db: Arc<Database>,
    pub options: poise::FrameworkOptions<(), Error>,
    pub shard_manager:
        std::sync::Mutex<Option<std::sync::Arc<tokio::sync::Mutex<serenity::ShardManager>>>>,
}

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn ready(&self, _: serenity::Context, ready: serenity::Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: serenity::Context, new_message: serenity::Message) {
        // FrameworkContext contains all data that poise::Framework usually manages
        let shard_manager = (*self.shard_manager.lock().unwrap()).clone().unwrap();
        let framework_data = poise::FrameworkContext {
            bot_id: serenity::UserId(846453852164587620), // TODO: Get the bot's actual user ID.
            options: &self.options,
            user_data: &(),
            shard_manager: &shard_manager,
        };

        poise::dispatch_event(framework_data, &ctx, &poise::Event::Message { new_message }).await;
    }

    // For slash commands or edit tracking to work, forward interaction_create and message_update
}

/// Basic ping command, useful for testing if the bot is running.
#[poise::command(slash_command)]
pub async fn ping(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
pub async fn age(
    ctx: poise::Context<'_, (), Error>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}
