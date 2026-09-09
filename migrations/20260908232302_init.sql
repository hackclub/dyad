CREATE TABLE hacker (
  user_id SERIAL PRIMARY KEY,
  hca_id text,
  email text,
  first_name text,
  hca_token text,
  hackatime_token text,
  first_time bool,
  join_at timestamp,
  last_login_at timestamp,
  slack_id text,
  friend_code int
);

CREATE TABLE project (
  project_id SERIAL PRIMARY KEY,
  name text,
  description text,
  screenshot_url text, 
  user1 int,
  user2 int,
  shipped_at timestamp,
  approved_at timestamp 
);

CREATE TABLE user_project (
  project_id SERIAL PRIMARY KEY,
  user_id int,
  hours_needed real,
  hours_logged real,
  confirmed_at timestamp,
  ship_ok_at timestamp,
  hackatime_project_names text[16]
);
