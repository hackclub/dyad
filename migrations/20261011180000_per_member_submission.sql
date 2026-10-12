ALTER TABLE user_project ADD COLUMN submitting_at timestamp;

ALTER TABLE project DROP COLUMN submitting_at;

ALTER TABLE hacker DROP COLUMN hca_token;

UPDATE user_project up SET ship_ok_at = NULL
FROM project p
WHERE p.project_id = up.project_id
  AND p.shipped_at IS NULL
  AND up.airtable_record_id IS NULL
  AND up.ship_ok_at IS NOT NULL;

CREATE TABLE wrong_friend_code (
  user_id int NOT NULL REFERENCES hacker(user_id) ON DELETE CASCADE,
  attempted_at timestamp NOT NULL
);

CREATE INDEX wrong_friend_code_user ON wrong_friend_code (user_id, attempted_at);
