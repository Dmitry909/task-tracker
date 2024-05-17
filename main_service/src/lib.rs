use axum::{
    body::{self, Bytes},
    extract::{DefaultBodyLimit, Query, Request, State},
    http::{header, request, response, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Form, Json, Router,
};
use chrono::NaiveDate;
use chrono::{Duration, Local};
use hex;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{de, forward_to_deserialize_any, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{database, postgres::PgPoolOptions, query, query_as, Pool, Postgres};
use std::{
    borrow::Borrow,
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    slice::RSplitN,
    str,
    sync::{Arc, RwLock},
};

pub async fn create_pool(database_url: &str) -> Pool<Postgres> {
    match PgPoolOptions::new().connect(&database_url).await {
        Ok(pool) => {
            return pool;
        }
        Err(err) => {
            println!("Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };
}

fn get_hash(password: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"dlkD7jQsiH");
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

struct AppState {
    pool: Pool<Postgres>,
}

#[derive(Debug)]
pub struct UsersModel {
    login: String,
    password_hash: String,
    first_name: Option<String>,
    second_name: Option<String>,
    birthday: Option<NaiveDate>,
    email: Option<String>,
    phone_number: Option<String>,
}

pub async fn create_app(users_db_url: &str, need_to_clear: bool) -> Router {
    let pool = create_pool(users_db_url).await;

    if need_to_clear {
        let _ = sqlx::query_as!(UsersModel, "TRUNCATE TABLE users",)
            .execute(&pool)
            .await;
    }

    let shared_state = Arc::new(AppState { pool });
    Router::new()
        .route("/signup", post(signup))
        // .route("/login", post(login))
        // .route("/update_user_data", put(update_user_data))
        .with_state(shared_state)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignupRequest {
    login: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    login: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Date {
    day: i32,
    month: i32,
    year: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserDataRequest {
    first_name: Option<String>,
    second_name: Option<String>,
    birthday: Option<Date>,
    email: Option<String>,
    phone_number: Option<String>,
}

fn check_login(login: &String) -> bool {
    if login.len() < 2 || login.len() > 20 {
        return false;
    }
    if !login
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return false;
    }
    return true;
}

fn check_password(password: &String) -> bool {
    if password.len() < 8 || password.len() > 30 {
        return false;
    }
    if !password.chars().all(|c| c.is_ascii()) {
        return false;
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return false;
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return false;
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    if !password
        .chars()
        .any(|c| !c.is_ascii_lowercase() && !c.is_ascii_uppercase() && !c.is_ascii_digit())
    {
        return false;
    }
    return true;
}

async fn signup(
    State(state): State<Arc<AppState>>,
    Json(input_payload): Json<SignupRequest>,
) -> Response {
    if !check_login(&input_payload.login) {
        return (
            StatusCode::NOT_ACCEPTABLE,
            "Login must be from 2 to 20 symbols and consist only of ascii lowercase letters and digist.",
        )
            .into_response();
    }
    if !check_password(&input_payload.password) {
        return (
            StatusCode::NOT_ACCEPTABLE,
            "Password must be from 8 to 30 symbols, consist only of ascii symbols and contain at least one lowercase, one uppercase and one digit.",
        )
            .into_response();
    }

    let query_result = sqlx::query_as!(
        UsersModel,
        "INSERT INTO users VALUES ($1, $2)",
        input_payload.login,
        get_hash(&input_payload.password),
    )
    .execute(&state.pool)
    .await;

    match query_result {
        Ok(_) => (StatusCode::CREATED).into_response(),
        Err(_) => (StatusCode::CONFLICT, "Username exists").into_response(),
    }
}
