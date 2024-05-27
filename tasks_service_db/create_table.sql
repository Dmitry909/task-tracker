CREATE TABLE IF NOT EXISTS tasks (
    task_id bigserial PRIMARY KEY,
    author_id bigint NOT NULL,
    text varchar NOT NULL
);
