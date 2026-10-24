DROP TABLE advisory, weather_rule, weather_snapshot, event_member, event, calendar_member, calendar,
           calendar_source, oauth_pending, oauth_account, member_avatar;
ALTER TABLE device DROP COLUMN settings;
ALTER TABLE household
    DROP COLUMN units,
    DROP COLUMN calendar_past_days,
    DROP COLUMN calendar_future_days,
    DROP COLUMN weather_retention_days,
    DROP COLUMN private_events_as_busy;
