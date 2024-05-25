CREATE TABLE IF NOT EXISTS users (
    id bigserial PRIMARY KEY,
    username varchar UNIQUE NOT NULL,
    password_hash varchar NOT NULL,
    first_name varchar(30),
    second_name varchar(30),
    birthday date,
    email varchar(255),
    phone_number varchar(20)
);
