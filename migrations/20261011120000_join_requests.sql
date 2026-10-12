CREATE TABLE join_request (
  project_id int NOT NULL REFERENCES project(project_id) ON DELETE CASCADE,
  user_id int NOT NULL REFERENCES hacker(user_id),
  first_time bool NOT NULL DEFAULT false,
  requested_at timestamp NOT NULL DEFAULT now(),
  PRIMARY KEY (project_id, user_id)
);

ALTER TABLE hacker ADD COLUMN hackatime_user_id bigint CONSTRAINT hacker_hackatime_user_id_unique UNIQUE;

ALTER TABLE project ADD COLUMN submitting_at timestamp;
