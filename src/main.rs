use anyhow::{anyhow, Context, Error};
use eval_bot::{age, ping, Data};
use mongodb::Database;
use poise::serenity_prelude as serenity;
use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::MongoDb] db: Database,
) -> ShuttlePoise<Data, Error> {
    // Get the discord token set in `Secrets.toml`
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    let framework = poise::Framework::<Data, _>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ping(), age()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(".".into()),
                additional_prefixes: vec![poise::Prefix::Literal("!")],
                mention_as_prefix: true,
                case_insensitive_commands: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .token(token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db })
            })
        })
        .build()
        .await
        .context("Failed to initialize Poise framework")?;

    Ok(framework.into())
}
