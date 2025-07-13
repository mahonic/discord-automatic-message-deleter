mod commands;
mod deps;
mod handler;
mod message_deletion;
mod persistence;

use chrono::TimeDelta;
use poise::serenity_prelude as serenity;
use std::env;
use std::sync::atomic::AtomicBool;

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");
    let loop_time = env::var("LOOP_TIME_SECONDS")
        .expect("Expected LOOP_TIME_SECONDS in the environment")
        .parse::<u64>()
        .expect("Expected LOOP_TIME_SECONDS to be an integer");
    // FYI if GUILDS intent is removed, then hanlder's cache_ready function won't ever trigger
    let intents = serenity::GatewayIntents::GUILD_MESSAGES | serenity::GatewayIntents::GUILDS; 

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::schedule_automatic_deletion_here(),
                commands::disable_automatic_deletion_here(),
                commands::trigger_message_deletion_here(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                for guild in &_ready.guilds {
                    poise::builtins::register_in_guild(
                        &ctx.http,
                        &framework.options().commands,
                        guild.id,
                    )
                    .await
                    .expect("Failed to register commands in a guild");
                }
                // poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(commands::Data {})
            })
        })
        .build();

    let mut client = serenity::Client::builder(&token, intents)
        .event_handler(handler::Handler {
            is_loop_running: AtomicBool::new(false),
            loop_period: TimeDelta::seconds(loop_time as i64),
        })
        .framework(framework)
        .await
        .expect("Err creating client");

    // Handle stopping the bot on ctrl + c signal
    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        println!("Exiting on CTRL + C interrupt.");
        shard_manager.shutdown_all().await;
    });

    // Start a single shard, and start listening to events.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
