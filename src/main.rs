use std::env;
use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions};
use poise::builtins::register_globally;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};

mod commands;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {}

#[tokio::main]
async fn main() {
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![commands::fetch()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}