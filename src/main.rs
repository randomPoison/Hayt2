use anyhow::anyhow;
use eval_bot::Bot;
use mongodb::Database;
use serenity::{
    async_trait, framework::StandardFramework, http::Http, model::prelude::Ready, prelude::*,
};
use shuttle_secrets::SecretStore;
use std::{collections::HashSet, sync::Arc};
use tracing::info;

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::MongoDb] db: Database,
) -> shuttle_serenity::ShuttleSerenity {
    std::env::set_var("RUST_BACKTRACE", "full");

    // Get the discord token set in `Secrets.toml`
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    // We will fetch your bot's owners and id
    let http = Http::new(&token);
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        }

        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Use the standard framework provided by serenity to structure the bot.
    let framework = StandardFramework::new().configure(|c| {
        c.with_whitespace(true)
            .on_mention(Some(bot_id))
            .prefix("~")
            // In this case, if "," would be first, a message would never
            // be delimited at ", ", forcing you to trim your arguments if you
            // want to avoid whitespaces at the start of each.
            .delimiters(vec![", ", ","])
            // Sets the bot's owners. These will be used for commands that
            // are owners only.
            .owners(owners)
    });

    // Build the client using the framework we setup.
    let intents = GatewayIntents::all();
    let client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<Bot>(Arc::new(Bot::new(db)))
        .await
        .expect("Err creating client");

    Ok(client.into())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}
