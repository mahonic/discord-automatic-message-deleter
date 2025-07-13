use crate::deps::get_persistence_manager;
use crate::message_deletion::delete_old_messages_from_channel;
use crate::persistence::ChannelSchedule;
use chrono::{TimeDelta, Utc};
use poise::CreateReply;

pub struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// TODO
//  I really don't like the fact that this can be so easily enabled.
//  It needs some additional warnings and checks to the user that this is being enabled I think.
//  To avoid accidental message deletion.
/// Schedules automatic message deletion for this channel.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_CHANNELS")]
pub async fn schedule_automatic_deletion_here(
    ctx: Context<'_>,
    #[description = "What's the maximum allowed messages' age before they will be deleted."]
    #[max = 839] // 14 days (max age of message that discord bots can delete) - 1 hour
    number_of_hours: u64,
    #[description = "Type \"I KNOW WHAT I'M DOING\" to actually schedule the deletion."]
    confirmation: String,
) -> Result<(), Error> {
    if confirmation != "I KNOW WHAT I'M DOING" {
        let reply = CreateReply::default()
            .reply(true)
            .ephemeral(true)
            .content("Do you _actually_ know what you're doing?");
        ctx.send(reply).await.unwrap();
        return Ok(());
    }

    ctx.defer().await?;

    let persistence_manager = get_persistence_manager(None);
    let schedule = ChannelSchedule {
        guild_id: ctx.guild_id().unwrap().get(),
        channel_id: ctx.channel_id().get(),
        scheduled_by_user_id: ctx.author().id.get(),
        max_message_age_hours: number_of_hours,
    };

    persistence_manager
        .set_schedule_for_channel(&schedule, Some(Utc::now()))
        .await;
    let reply = CreateReply::default()
        .reply(true)
        .content("Deletion schedule set!");
    ctx.send(reply).await.unwrap();
    Ok(())
}

/// Removes this channel from automatic message deletion schedule.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_CHANNELS")]
pub async fn disable_automatic_deletion_here(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let persistence_manager = get_persistence_manager(None);
    persistence_manager
        .clear_schedule_for_channel(
            ctx.channel_id().get(),
            ctx.author().id.get(),
            Some(Utc::now()),
        )
        .await;
    let reply = CreateReply::default()
        .reply(true)
        .content("Deletion schedule removed!");
    ctx.send(reply).await.unwrap();
    Ok(())
}
/// Trigger message deletion now
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_CHANNELS")]
pub async fn trigger_message_deletion_here(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let persistence_manager = get_persistence_manager(None);
    let channel_schedule = persistence_manager
        .get_schedule_for_channel(ctx.channel_id().get())
        .await;
    let channel_schedule = match channel_schedule {
        Some(schedule) => schedule,
        None => {
            let reply = CreateReply::default()
                .reply(true)
                .ephemeral(true)
                .content("Channel has no active schedule. Please set a deletion schedule first.");
            ctx.send(reply).await.unwrap();
            return Ok(());
        }
    };
    delete_old_messages_from_channel(
        ctx.guild_id().unwrap(),
        ctx.channel_id(),
        TimeDelta::hours(channel_schedule.max_message_age_hours as i64),
        &ctx.serenity_context(),
        None,
        true,
    )
    .await;
    let reply = CreateReply::default().reply(true).content("Done");
    ctx.send(reply).await.unwrap();
    Ok(())
}
