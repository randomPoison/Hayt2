use anyhow::anyhow;
use eval_bot::{age, ping, Handler};
use mongodb::Database;
use poise::serenity_prelude as serenity;
use shuttle_secrets::SecretStore;
use std::sync::Arc;

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::MongoDb] db: Database,
) -> shuttle_serenity::ShuttleSerenity {
    std::env::set_var("RUST_BACKTRACE", "1");

    // Get the discord token set in `Secrets.toml`
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    let mut handler = Handler {
        db: Arc::new(db),
        options: poise::FrameworkOptions {
            commands: vec![ping(), age()],
            ..Default::default()
        },
        shard_manager: std::sync::Mutex::new(None),
    };
    poise::set_qualified_names(&mut handler.options.commands); // some setup

    let handler = Arc::new(handler);
    let intents = serenity::GatewayIntents::non_privileged();
    let client = serenity::Client::builder(token, intents)
        .event_handler_arc(handler.clone())
        .await
        .expect("Failed to create `Client`");

    // Initialize the shard manager once the `Client` instance has been initialized.
    *handler.shard_manager.lock().unwrap() = Some(client.shard_manager.clone());

    Ok(client.into())
}
