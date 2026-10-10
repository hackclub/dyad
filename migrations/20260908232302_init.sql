CREATE TABLE hacker (
  user_id SERIAL PRIMARY KEY,
  hca_id text NOT NULL UNIQUE,
  email text,
  first_name text,
  hca_token text,
  hackatime_token text,
  join_at timestamp,
  last_login_at timestamp,
  slack_id text,
  friend_code int UNIQUE
);

CREATE TABLE project (
  project_id SERIAL PRIMARY KEY,
  name text,
  description text,
  screenshot_url text,
  code_url text,
  playable_url text,
  game text,
  user1 int REFERENCES hacker(user_id),
  user2 int REFERENCES hacker(user_id),
  created_at timestamp,
  shipped_at timestamp,
  approved_at timestamp
);

CREATE TABLE user_project (
  project_id int NOT NULL REFERENCES project(project_id) ON DELETE CASCADE,
  user_id int NOT NULL REFERENCES hacker(user_id),
  first_time bool NOT NULL DEFAULT false,
  hours_needed real,
  hours_logged real,
  confirmed_at timestamp,
  ship_ok_at timestamp,
  hackatime_project_names text[] NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, user_id)
);
