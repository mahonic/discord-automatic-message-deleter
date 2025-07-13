use crate::persistence::Persistence;
use chrono::{DateTime, TimeDelta, Utc};
use serenity::all::{ChannelId, GuildId, MessageId};
use serenity::futures::StreamExt;
use serenity::prelude::Context;

const MAX_DELETION_COUNT: usize = 100;
const MAX_MESSAGE_AGE_DAYS: i64 = 14;

pub async fn delete_messages_according_to_schedule<P: AsRef<Persistence>, C: AsRef<Context>>(
    asked_at: Option<DateTime<Utc>>,
    persistence_manager: P,
    context: C,
) {
    let asked_at = asked_at.unwrap_or_else(|| Utc::now());
    let persistence_manager = persistence_manager.as_ref();
    let context = context.as_ref();

    let active_schedule = persistence_manager.get_active_schedules().await;

    for schedule in active_schedule {
        delete_old_messages_from_channel(
            schedule.guild_id.into(),
            schedule.channel_id.into(),
            TimeDelta::hours(schedule.max_message_age_hours as i64),
            context,
            Some(asked_at),
            false,
        )
        .await;
    }
}

pub async fn delete_old_messages_from_channel(
    guild_id: GuildId,
    channel_id: ChannelId,
    max_message_age: TimeDelta,
    ctx: &Context,
    asked_at: Option<DateTime<Utc>>,
    verbose: bool,
) {
    let asked_at = asked_at.unwrap_or_else(Utc::now);
    let (messages, first_message_past_deletion_range) = get_messages_for_deletion(
        channel_id.clone(),
        max_message_age.clone(),
        ctx,
        asked_at.clone(),
    )
    .await;

    let deleted_messages_count = delete_messages(&messages, channel_id.clone(), &ctx).await;

    if deleted_messages_count > 0 && verbose {
        println!("Deleted {} messages", deleted_messages_count);
        channel_id
            .say(
                &ctx.http,
                format!("Deleted {} messages", deleted_messages_count),
            )
            .await
            .unwrap();
    }

    if let Some(first_message_past_deletion_range) = first_message_past_deletion_range {
        channel_id.say(
            &ctx.http,
            format!("Found messages that are too old for deletion.\
                 \nAnything past this [message](https://discord.com/channels/{}/{}/{}) needs to be deleted manually.",
                    guild_id, channel_id, first_message_past_deletion_range)
        ).await.unwrap();
    }
}

async fn get_messages_for_deletion(
    channel_id: ChannelId,
    age_deletion_threshold: TimeDelta,
    ctx: &Context,
    asked_at: DateTime<Utc>,
) -> (Vec<MessageId>, Option<MessageId>) {
    let mut messages: Vec<MessageId> = Vec::new();
    let mut first_message_past_deletion_range: Option<MessageId> = None;
    let mut messages_iter = channel_id.messages_iter(&ctx).boxed();
    while let Some(message_result) = messages_iter.next().await {
        match message_result {
            Ok(message) => {
                let message_age: TimeDelta = asked_at - message.timestamp.to_utc();
                if message_age >= TimeDelta::days(MAX_MESSAGE_AGE_DAYS) {
                    first_message_past_deletion_range = Some(message.id);
                    break;
                } else if message_age >= age_deletion_threshold {
                    messages.push(message.id);
                }
            }
            Err(error) => eprintln!("Uh oh! Error: {}", error),
        }
    }
    (messages, first_message_past_deletion_range)
}

async fn delete_messages<M: AsRef<Vec<MessageId>>>(
    messages: M,
    channel_id: ChannelId,
    ctx: &Context,
) -> usize {
    let messages = messages.as_ref();
    let mut deleted_messages_count = 0;
    for slice in messages.chunks(MAX_DELETION_COUNT) {
        channel_id.delete_messages(&ctx.http, slice).await.unwrap();
        deleted_messages_count += slice.len();
    }

    deleted_messages_count
}

// soon 👏
// https://doc.rust-lang.org/nightly/std/prelude/v1/attr.define_opaque.html
// https://github.com/rust-lang/rust/issues/63063
// type DeleteMessages = impl AsyncFn(Vec<i32>) -> Result<(), Box<dyn std::error::Error>>;
//
//
// async fn some_factory() -> DeleteMessages {
//     async move |messages: Vec<i32>| {
//         Ok(())
//     }
// }

fn delete_messages_test<M: AsRef<Vec<i32>>>(messages_to_be_deleted: M) -> i32 {
    let messages_to_be_deleted = messages_to_be_deleted.as_ref();
    let mut deletion_calls_count = 0;
    for _slice in messages_to_be_deleted.chunks(MAX_DELETION_COUNT) {
        deletion_calls_count += 1;
    }
    deletion_calls_count
}

#[tokio::test]
async fn test_iter() {
    let message_count = [0, 1, 2, 99, 100, 101, 200, 201];
    let expected_calls = [0, 1, 1, 1, 1, 2, 2, 3];
    for i in 0..message_count.len() {
        let message_count = message_count[i];
        let expected_calls = expected_calls[i];
        let mut some_data: Vec<i32> = Vec::new();
        for data in 0..message_count {
            some_data.push(data);
        }
        let result = delete_messages_test(some_data);
        assert_eq!(
            result, expected_calls,
            "Message count = {} expected calls = {}",
            message_count, expected_calls
        );
    }
}
