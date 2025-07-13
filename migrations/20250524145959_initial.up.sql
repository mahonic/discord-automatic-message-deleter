-- Add up migration script here
CREATE TABLE channel_schedule (
    id BIGSERIAL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    deleted_by_user_id BIGINT DEFAULT NULL,
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    scheduled_by_user_id BIGINT NOT NULL,
    max_message_age_hours BIGINT NOT NULL
);