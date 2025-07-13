use crate::deps::get_persistence_manager;
use crate::message_deletion::delete_messages_according_to_schedule;
use chrono::Duration;
use poise::async_trait;
use poise::serenity_prelude::EventHandler;
use serenity::all::{Context, GuildId, Ready};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Handler {
    pub(crate) is_loop_running: AtomicBool,
    pub(crate) loop_period: Duration,
}

#[async_trait]
impl EventHandler for Handler {
    // We use the cache_ready event just in case some cache operation is required in whatever use
    // case you have for this.
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("Cache built successfully!");

        // It's safe to clone Context, but Arc is cheaper for this use case.
        // Untested claim, just theoretically. :P
        let ctx = Arc::new(ctx);

        // We need to check that the loop is not already running when this event triggers, as this
        // event triggers every time the bot enters or leaves a guild, along every time the ready
        // shard event triggers.
        //
        // An AtomicBool is used because it doesn't require a mutable reference to be changed, as
        // we don't have one due to self being an immutable reference.
        if !self.is_loop_running.load(Ordering::Relaxed) {
            let persistence_manager = get_persistence_manager(None);
            let loop_period = self.loop_period.clone();
            tokio::spawn(async move {
                loop {
                    delete_messages_according_to_schedule(None, &persistence_manager, &ctx).await;
                    tokio::time::sleep(loop_period.to_std().unwrap()).await;
                }
            });
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        for guild in &ready.guilds {
            println!("I'm present on guild {}!", guild.id);
        }
    }
}
