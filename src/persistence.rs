use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};

pub struct ChannelSchedule {
    pub guild_id: u64,
    pub channel_id: u64,
    pub scheduled_by_user_id: u64,
    pub max_message_age_hours: u64,
}

pub struct Persistence {
    pub database_url: String,
}

impl AsRef<Persistence> for Persistence {
    fn as_ref(&self) -> &Persistence {
        &self
    }
}

// TODO proper error handling
//  Currently we just unwrap everything and hope for the best.
impl Persistence {
    pub async fn set_schedule_for_channel(
        &self,
        schedule: &ChannelSchedule,
        asked_at: Option<DateTime<Utc>>,
    ) -> () {
        // We're doing a silly here.
        // Postgres doesn't support unsigned 64-bit integers. So we convert them signed integers.
        // And will have to do the revers everytime we read from the db.
        let guild_id = schedule.guild_id as i64;
        let channel_id = schedule.channel_id as i64;
        let scheduled_by_user_id = schedule.scheduled_by_user_id as i64;
        let max_message_age_hours = schedule.max_message_age_hours as i64;
        let asked_at = asked_at.unwrap_or_else(|| Utc::now());

        let pool = self.get_db_pool().await;
        // Doing it in a transaction to avoid race conditions.
        // This is a low traffic application, so we don't really need to worry about latency here.
        {
            let mut transaction = pool.begin().await.unwrap();
            let result = sqlx::query!(
                r#"
                INSERT INTO channel_schedule (guild_id, channel_id, scheduled_by_user_id, max_message_age_hours)
                VALUES ($1, $2, $3, $4)
                RETURNING ID
                "#,
                &guild_id,
                &channel_id,
                &scheduled_by_user_id,
                &max_message_age_hours,
            )
                .fetch_one(&mut *transaction)
                .await
                .unwrap();
            let _ = sqlx::query!(
                r#"
                 UPDATE channel_schedule
                 SET deleted_at = $1, deleted_by_user_id = $2
                 WHERE channel_id = $3 AND id != $4
                 "#,
                &asked_at,
                &scheduled_by_user_id,
                &channel_id,
                &result.id
            )
            .execute(&mut *transaction)
            .await
            .unwrap();
            transaction.commit().await.unwrap();
        }
    }

    pub async fn clear_schedule_for_channel(
        &self,
        channel_id: u64,
        deleter_id: u64,
        asked_at: Option<DateTime<Utc>>,
    ) -> () {
        let channel_id = channel_id as i64;
        let deleter_id = deleter_id as i64;
        let asked_at = asked_at.unwrap_or_else(|| Utc::now());
        let _ = sqlx::query!(
            r#"
            UPDATE channel_schedule
            SET deleted_at = $1, deleted_by_user_id = $2
            WHERE channel_id = $3 AND deleted_at IS NULL
            "#,
            &asked_at,
            &deleter_id,
            &channel_id
        )
        .execute(&self.get_db_pool().await)
        .await
        .unwrap();
    }

    pub async fn get_active_schedules(&self) -> Vec<ChannelSchedule> {
        let results = sqlx::query!(
            r#"
            SELECT guild_id, channel_id, scheduled_by_user_id, max_message_age_hours
            FROM channel_schedule
            WHERE deleted_at IS NULL
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.get_db_pool().await)
        .await
        .unwrap();
        results
            .into_iter()
            .map(|result| ChannelSchedule {
                guild_id: result.guild_id as u64,
                channel_id: result.channel_id as u64,
                scheduled_by_user_id: result.scheduled_by_user_id as u64,
                max_message_age_hours: result.max_message_age_hours as u64,
            })
            .collect()
    }

    pub async fn get_schedule_for_channel(&self, channel_id: u64) -> Option<ChannelSchedule> {
        let channel_id = channel_id as i64;
        let result = sqlx::query!(
            r#"
            SELECT guild_id, channel_id, scheduled_by_user_id, max_message_age_hours
            FROM channel_schedule
            WHERE deleted_at IS NULL AND channel_id = $1
            "#,
            &channel_id
        )
        .fetch_optional(&self.get_db_pool().await)
        .await
        .unwrap();
        if result.is_none() {
            return None;
        }
        let result = result.unwrap();
        Some(ChannelSchedule {
            guild_id: result.guild_id as u64,
            channel_id: result.channel_id as u64,
            scheduled_by_user_id: result.scheduled_by_user_id as u64,
            max_message_age_hours: result.max_message_age_hours as u64,
        })
    }

    async fn get_db_pool(&self) -> Pool<Postgres> {
        let pool = sqlx::PgPool::connect(&self.database_url.as_str())
            .await
            .unwrap();
        pool
    }
}
