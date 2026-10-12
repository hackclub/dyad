CREATE TABLE dm_log (
  sender int NOT NULL REFERENCES hacker(user_id) ON DELETE CASCADE,
  recipient int NOT NULL REFERENCES hacker(user_id) ON DELETE CASCADE,
  sent_at timestamp NOT NULL
);

CREATE INDEX dm_log_pair ON dm_log (sender, recipient, sent_at);

CREATE TABLE hackatime_account (
  hackatime_user_id bigint PRIMARY KEY,
  user_id int NOT NULL REFERENCES hacker(user_id) ON DELETE CASCADE
);

INSERT INTO hackatime_account (hackatime_user_id, user_id)
SELECT hackatime_user_id, user_id FROM hacker WHERE hackatime_user_id IS NOT NULL;
