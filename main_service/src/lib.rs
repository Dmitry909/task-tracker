use axum::{
    extract::State,
    http::{request, response, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Local;
use chrono::NaiveDate;
use hex;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use proto::task_service_client::TaskServiceClient;
use proto::{
    CreateTaskRequest, DeleteTaskRequest, GetTaskRequest, ListTasksRequest, UpdateTaskRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::{borrow::Borrow, str, sync::Arc, thread, time::Duration};
use tonic;

pub mod proto {
    tonic::include_proto!("tasks");
}

pub async fn create_pool(database_url: &str) -> Pool<Postgres> {
    let mut attempts = 0;
    let max_attempts = 5;

    while attempts < max_attempts {
        match PgPoolOptions::new().connect(&database_url).await {
            Ok(pool) => return pool,
            Err(err) => {
                println!(
                    "Attempt {}: Failed to connect to the database: {:?}",
                    attempts + 1,
                    err
                );
                attempts += 1;
                if attempts < max_attempts {
                    thread::sleep(Duration::from_secs(5)); // wait for 5 seconds before retrying
                }
            }
        }
    }

    println!(
        "Failed to connect to the database after {} attempts.",
        max_attempts
    );
    std::process::exit(1);
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
    id: u64,
    username: String,
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
        let _ = sqlx::query("TRUNCATE TABLE users").execute(&pool).await;
    }

    let shared_state = Arc::new(AppState { pool });
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/personal_data", put(update_personal_data))
        .route("/personal_data", get(get_personal_data))
        .route("/create_task", post(create_task))
        .route("/update_task", put(update_task))
        .route("/delete_task", delete(delete_task))
        .route("/get_task", get(get_task))
        .route("/list_tasks", get(list_tasks))
        .with_state(shared_state)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignupRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Date {
    day: u32,
    month: u32,
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
    if !check_login(&input_payload.username) {
        return (
            StatusCode::NOT_ACCEPTABLE,
            "Login must be from 2 to 20 symbols and consist only of ascii lowercase letters and digist.",
        )
            .into_response();
    }
    if !check_password(&input_payload.password) {
        return (
            StatusCode::NOT_ACCEPTABLE,
            "Password must be from 8 to 30 symbols, consist only of ascii symbols and contain at least one lowercase, one uppercase, one digit and one symbol.",
        )
            .into_response();
    }

    let query_result = sqlx::query(&format!(
        "INSERT INTO users (username, password_hash) VALUES ('{}', '{}')",
        input_payload.username,
        get_hash(&input_payload.password)
    ))
    .execute(&state.pool)
    .await;

    match query_result {
        Ok(_) => (StatusCode::CREATED).into_response(),
        Err(_) => (StatusCode::CONFLICT, "Username exists").into_response(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenData {
    username: String,
    exp: usize,
}

fn generate_token(username: &String) -> String {
    let secret = b"my_secret_key_d47fjs&w3)wj";
    let token_data = TokenData {
        username: username.clone(),
        exp: (Local::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };
    let encoding_key = EncodingKey::from_secret(secret);
    encode(&Header::default(), &token_data, &encoding_key).unwrap()
}

fn decode_token(
    token: &str,
) -> Result<jsonwebtoken::TokenData<TokenData>, jsonwebtoken::errors::Error> {
    let secret = b"my_secret_key_d47fjs&w3)wj";
    return decode::<TokenData>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    );
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(input_payload): Json<LoginRequest>,
) -> Response {
    let query_result = sqlx::query(&format!(
        "SELECT COUNT(*) FROM users WHERE username='{}' and password_hash='{}'",
        &input_payload.username,
        &get_hash(&input_payload.password),
    ))
    .fetch_one(&state.pool)
    .await;

    match query_result {
        Ok(count) => {
            let q: Result<i64, sqlx::Error> = count.try_get(0);
            match q {
                Ok(count) => match count {
                    1 => {}
                    _ => {
                        return (StatusCode::UNAUTHORIZED).into_response();
                    }
                },
                Err(_) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            }
        }
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let token = generate_token(&input_payload.username);
    (StatusCode::OK, [("Authorization", token)]).into_response()
}

enum CheckAuthorizationResult {
    Username(String),
    NoToken,
    Invalid,
}

async fn check_authorization(headers: HeaderMap) -> CheckAuthorizationResult {
    if !headers.contains_key("Authorization") {
        return CheckAuthorizationResult::NoToken;
    }
    let token = headers["Authorization"].to_str().unwrap();
    let decoded_token = match decode_token(token) {
        Ok(c) => c.claims,
        Err(_) => {
            return CheckAuthorizationResult::Invalid;
        }
    };

    return CheckAuthorizationResult::Username(decoded_token.username);
}

async fn update_personal_data(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input_payload): Json<UpdateUserDataRequest>,
) -> Response {
    let username = match check_authorization(headers).await {
        CheckAuthorizationResult::Username(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let mut set_vector = Vec::new();
    match input_payload.first_name {
        Some(first_name) => {
            set_vector.push(format!("first_name='{}'", first_name));
        }
        None => {}
    };
    match input_payload.second_name {
        Some(second_name) => {
            set_vector.push(format!("second_name='{}'", second_name));
        }
        None => {}
    };
    match input_payload.birthday {
        Some(birthday) => {
            let date_opt = NaiveDate::from_ymd_opt(birthday.year, birthday.month, birthday.day);
            match date_opt {
                Some(date) => {
                    set_vector.push(format!("birthday='{}', ", date));
                }
                None => {
                    return (StatusCode::NOT_ACCEPTABLE, "Incorrect birthdate").into_response();
                }
            };
        }
        None => {}
    };
    match input_payload.email {
        Some(email) => {
            set_vector.push(format!("email='{}'", email));
        }
        None => {}
    };
    match input_payload.phone_number {
        Some(phone_number) => {
            set_vector.push(format!("phone_number='{}'", phone_number));
        }
        None => {}
    };

    let query = format!(
        "UPDATE users SET {} WHERE username='{}' RETURNING *",
        set_vector.join(", "),
        username
    );

    let query_result = sqlx::query(&query).fetch_optional(&state.pool).await;
    match query_result {
        Ok(query_result_opt) => match query_result_opt {
            Some(_) => (StatusCode::OK).into_response(),
            None => (StatusCode::NOT_FOUND).into_response(),
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetUserDataResponse {
    first_name: Option<String>,
    second_name: Option<String>,
    email: Option<String>,
    phone_number: Option<String>,
}

async fn get_personal_data(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let username = match check_authorization(headers).await {
        CheckAuthorizationResult::Username(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let query_result = sqlx::query(&format!(
        "SELECT first_name, second_name, email, phone_number FROM users WHERE username='{}'",
        &username
    ))
    .fetch_optional(&state.pool)
    .await;

    match query_result {
        Ok(opt) => match opt {
            Some(row) => {
                let first_name: Option<String> = row.get("first_name");
                let second_name: Option<String> = row.get("second_name");
                let email: Option<String> = row.get("email");
                let phone_number: Option<String> = row.get("phone_number");
                let result = GetUserDataResponse {
                    first_name,
                    second_name,
                    email,
                    phone_number,
                };
                (StatusCode::OK, Json(result)).into_response()
            }
            None => (StatusCode::NOT_FOUND).into_response(),
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskRequest1 {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTaskRequest1 {
    task_id: i64,
    new_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteTaskRequest1 {
    task_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTaskRequest1 {
    task_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasksRequest1 {
    offset: i64,
    limit: i64,
}

async fn create_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input_payload): Json<CreateTaskRequest1>,
) -> Response {
    let username = match check_authorization(headers).await {
        CheckAuthorizationResult::Username(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let url = "http://localhost:23456";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::CreateTaskRequest {
        author_id: 1,
        text: input_payload.text,
    };
    let request = tonic::Request::new(req);
    let response = match client.create_task(request).await {
        Ok(response) => response,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    println!("{}", response.get_ref().task_id);

    (StatusCode::OK).into_response()
}

async fn update_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input_payload): Json<UpdateTaskRequest1>,
) -> Response {
    let username = match check_authorization(headers).await {
        CheckAuthorizationResult::Username(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    (StatusCode::OK).into_response()
}

async fn delete_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input_payload): Json<DeleteTaskRequest1>,
) -> Response {
    let username = match check_authorization(headers).await {
        CheckAuthorizationResult::Username(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    (StatusCode::OK).into_response()
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input_payload): Json<GetTaskRequest1>,
) -> Response {
    let username = match check_authorization(headers).await {
        CheckAuthorizationResult::Username(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    (StatusCode::OK).into_response()
}

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input_payload): Json<ListTasksRequest1>,
) -> Response {
    let username = match check_authorization(headers).await {
        CheckAuthorizationResult::Username(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    (StatusCode::OK).into_response()
}
