use anyhow::Error;
use mongodb::Database;
use poise::serenity_prelude as serenity;

pub mod bug;
pub mod todo;

type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    pub db: Database,
}

/// Basic ping command, useful for testing if the bot is running.
#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
pub async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}
