ALTER TABLE hacker ADD COLUMN invite_token text NOT NULL UNIQUE DEFAULT replace(gen_random_uuid()::text, '-', '');

CREATE UNIQUE INDEX project_one_partnerless ON project (user1) WHERE user2 IS NULL;

ALTER TABLE user_project DROP COLUMN last_seen_hours;
ALTER TABLE user_project DROP COLUMN last_active_at;
