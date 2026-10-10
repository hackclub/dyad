ALTER TABLE hacker ADD COLUMN reminders_opt_out bool NOT NULL DEFAULT false;

ALTER TABLE project ADD COLUMN review_status text;

ALTER TABLE user_project ADD COLUMN airtable_record_id text;
ALTER TABLE user_project ADD COLUMN last_seen_hours real;
ALTER TABLE user_project ADD COLUMN last_active_at timestamp;
ALTER TABLE user_project ADD COLUMN nudged_at timestamp;
