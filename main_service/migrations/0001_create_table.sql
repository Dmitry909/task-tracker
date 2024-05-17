CREATE TABLE IF NOT EXISTS users (
    login varchar(20) PRIMARY KEY NOT NULL,
    password_hash varchar(1000) NOT NULL,
    first_name varchar(30),
    second_name varchar(30),
    birthday date,
    email varchar(255),
    phone_number varchar(20)
);
