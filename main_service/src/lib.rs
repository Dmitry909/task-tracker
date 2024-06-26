use axum::{
    extract::State,
    http::{request, response, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Error, Json, Router,
};
use chrono::Local;
use chrono::NaiveDate;
use hex;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use proto::{stat_service_client::StatServiceClient, HealthcheckRequest, HealthcheckResponse};
use proto::{task_service_client::TaskServiceClient, CreateTaskResponse};
use proto::{
    CreateTaskRequest, DeleteTaskRequest, GetTaskRequest, ListTasksRequest, UpdateTaskRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::{borrow::Borrow, str, sync::Arc, thread, time::Duration};
use tonic;
use tracing_subscriber::field::RecordFields;

pub mod proto {
    tonic::include_proto!("common");
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
        .route("/like", post(like))
        .route("/view", post(view))
        .route("/healthcheck_stat", get(healthcheck_stat))
        .route("/likes_and_views", get(likes_and_views))
        .route("/most_popular_tasks", get(most_popular_tasks))
        .route("/most_popular_users", get(most_popular_users))
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
    id: i64,
    username: String,
    exp: usize,
}

fn generate_token(id: i64, username: &String) -> String {
    let secret = b"my_secret_key_d47fjs&w3)wj";
    let token_data = TokenData {
        id,
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
        "SELECT * FROM users WHERE username='{}' and password_hash='{}'",
        &input_payload.username,
        &get_hash(&input_payload.password),
    ))
    .fetch_optional(&state.pool)
    .await;

    let row = match query_result {
        Ok(row_opt) => match row_opt {
            Some(row) => {
                let id: i64 = row.try_get("id").unwrap();
                let username: String = row.try_get("username").unwrap();
                (id, username)
            }
            None => {
                return (StatusCode::UNAUTHORIZED).into_response();
            }
        },
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let token = generate_token(row.0, &row.1);
    (StatusCode::OK, [("Authorization", token)]).into_response()
}

enum CheckAuthorizationResult {
    IdAndUsername((i64, String)),
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

    return CheckAuthorizationResult::IdAndUsername((decoded_token.id, decoded_token.username));
}

async fn update_personal_data(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input_payload): Json<UpdateUserDataRequest>,
) -> Response {
    let id_and_username = match check_authorization(headers).await {
        CheckAuthorizationResult::IdAndUsername(username) => username,
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
        id_and_username.1
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
    let id_and_username = match check_authorization(headers).await {
        CheckAuthorizationResult::IdAndUsername(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let query_result = sqlx::query(&format!(
        "SELECT first_name, second_name, email, phone_number FROM users WHERE username='{}'",
        &id_and_username.1
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
pub struct CreateTaskResponse1 {
    task_id: i64,
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
pub struct GetTaskResponse1 {
    task_id: i64,
    author_id: i64,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasksRequest1 {
    user_id: i64,
    offset: i64,
    limit: i64,
}

async fn create_task(
    headers: HeaderMap,
    Json(input_payload): Json<CreateTaskRequest1>,
) -> Response {
    let id_and_username = match check_authorization(headers).await {
        CheckAuthorizationResult::IdAndUsername(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let url = "http://tasks_service:50051";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::CreateTaskRequest {
        author_id: id_and_username.0,
        text: input_payload.text,
    };
    let request = tonic::Request::new(req);
    let response = match client.create_task(request).await {
        Ok(response) => response,
        Err(e) => {
            println!("Error creating task: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let resp = CreateTaskResponse1 {
        task_id: response.get_ref().task_id,
    };
    (StatusCode::CREATED, Json(resp)).into_response()
}

async fn update_task(
    headers: HeaderMap,
    Json(input_payload): Json<UpdateTaskRequest1>,
) -> Response {
    let id_and_username = match check_authorization(headers).await {
        CheckAuthorizationResult::IdAndUsername(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let url = "http://tasks_service:50051";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::UpdateTaskRequest {
        user_id: id_and_username.0,
        task_id: input_payload.task_id,
        new_text: input_payload.new_text,
    };
    let request = tonic::Request::new(req);
    match client.update_task(request).await {
        Ok(_) => {}
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    (StatusCode::OK).into_response()
}

async fn delete_task(
    headers: HeaderMap,
    Json(input_payload): Json<DeleteTaskRequest1>,
) -> Response {
    let id_and_username = match check_authorization(headers).await {
        CheckAuthorizationResult::IdAndUsername(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let url = "http://tasks_service:50051";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::DeleteTaskRequest {
        user_id: id_and_username.0,
        task_id: input_payload.task_id,
    };
    let request = tonic::Request::new(req);
    match client.delete_task(request).await {
        Ok(_) => {}
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    (StatusCode::OK).into_response()
}

async fn get_task(Json(input_payload): Json<GetTaskRequest1>) -> Response {
    let url = "http://tasks_service:50051";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::GetTaskRequest {
        task_id: input_payload.task_id,
    };
    let request = tonic::Request::new(req);
    let response = match client.get_task(request).await {
        Ok(response) => response,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let resp = GetTaskResponse1 {
        task_id: response.get_ref().task_id,
        author_id: response.get_ref().author_id,
        text: response.get_ref().text.clone(),
    };
    (StatusCode::CREATED, Json(resp)).into_response()
}

async fn list_tasks(Json(input_payload): Json<ListTasksRequest1>) -> Response {
    let url = "http://tasks_service:50051";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::ListTasksRequest {
        user_id: input_payload.user_id,
        offset: input_payload.offset,
        limit: input_payload.limit,
    };
    let request = tonic::Request::new(req);
    let response = match client.list_tasks(request).await {
        Ok(response) => response,
        Err(e) => {
            println!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let tasks: Vec<GetTaskResponse1> = response
        .get_ref()
        .clone()
        .tasks
        .into_iter()
        .map(|task| GetTaskResponse1 {
            task_id: task.task_id,
            author_id: task.author_id,
            text: task.text,
        })
        .collect();

    (StatusCode::OK, Json(tasks)).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LikeOrViewRequest1 {
    task_id: i64,
}

async fn like(headers: HeaderMap, Json(input_payload): Json<LikeOrViewRequest1>) -> Response {
    let id_and_username = match check_authorization(headers).await {
        CheckAuthorizationResult::IdAndUsername(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let url = "http://tasks_service:50051";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let req = proto::SendLikeOrViewRequest {
        task_id: input_payload.task_id,
        liker_id: id_and_username.0,
    };
    let request = tonic::Request::new(req);
    match client.send_like(request).await {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    (StatusCode::OK).into_response()
}

async fn view(headers: HeaderMap, Json(input_payload): Json<LikeOrViewRequest1>) -> Response {
    let id_and_username = match check_authorization(headers).await {
        CheckAuthorizationResult::IdAndUsername(username) => username,
        CheckAuthorizationResult::NoToken => {
            return (StatusCode::UNAUTHORIZED, "Token is missing").into_response();
        }
        CheckAuthorizationResult::Invalid => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    let url = "http://tasks_service:50051";
    let mut client = match TaskServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let req = proto::SendLikeOrViewRequest {
        task_id: input_payload.task_id,
        liker_id: id_and_username.0,
    };
    let request = tonic::Request::new(req);
    match client.send_view(request).await {
        Ok(_) => {}
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    (StatusCode::OK).into_response()
}

async fn healthcheck_stat() -> Response {
    let url = "http://stat_service:50052";
    let mut client = match StatServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("!!! Error creating client");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    eprintln!("@@@ Client created");

    let req = proto::HealthcheckRequest { a: 4 };
    let request = tonic::Request::new(req);
    match client.healthcheck(request).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("!!! Error connecting by grpc: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    (StatusCode::OK).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LikesAndViewsRequest1 {
    task_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LikesAndViewsResponse1 {
    task_id: i64,
    likes_count: i64,
    views_count: i64,
}

async fn likes_and_views(Json(input_payload): Json<LikesAndViewsRequest1>) -> Response {
    let url = "http://stat_service:50052";
    let mut client = match StatServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    eprintln!("Client created");
    let req = proto::GetLikesAndViewsRequest {
        task_id: input_payload.task_id,
    };
    eprintln!("req created");
    let request = tonic::Request::new(req);
    eprintln!("request created");
    let response = match client.get_likes_and_views(request).await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    eprintln!("response got");

    let resp = LikesAndViewsResponse1 {
        task_id: response.get_ref().task_id,
        likes_count: response.get_ref().likes_count,
        views_count: response.get_ref().views_count,
    };

    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Top5TasksRequest1 {
    sort_by_likes: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Top5TasksResponse1 {
    task_id: i64,
    author_username: String,
    likes_count: i64,
    views_count: i64,
}

async fn get_username_by_id(state: &Arc<AppState>, user_id: i64) -> Option<String> {
    eprintln!("SELECT * FROM users WHERE id='{}'", user_id);
    let query_result = sqlx::query(&format!("SELECT * FROM users WHERE id='{}'", user_id,))
        .fetch_optional(&state.pool)
        .await;

    match query_result {
        Ok(row_opt) => match row_opt {
            Some(row) => {
                eprintln!("Got row PgRow");
                let username: String = row.try_get("username").unwrap();
                Some(username)
            }
            None => {
                eprintln!("234");
                None
            },
        },
        Err(_) => {
            eprintln!("345");
            None
        },
    }
}

async fn most_popular_tasks(
    State(state): State<Arc<AppState>>,
    Json(input_payload): Json<Top5TasksRequest1>,
) -> Response {
    let url = "http://stat_service:50052";
    let mut client = match StatServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::GetTop5PostsRequest {
        sort_by_likes: input_payload.sort_by_likes,
    };
    eprintln!("req created");
    let request = tonic::Request::new(req);
    eprintln!("request created");
    let response = match client.get_top5_posts(request).await {
        Ok(response) => response,
        Err(e) => {
            println!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    eprintln!("response got");
    let mut tasks: Vec<Top5TasksResponse1> = vec![];
    for record in response.get_ref().clone().posts.iter() {
        // let username = match get_username_by_id(&state, record.author_id).await {
        //     Some(username) => username,
        //     None => {
        //         eprintln!("Couldn't get username of {}", record.author_id);
        //         return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        //     }
        // };
        let username = "not implemented".to_string();
        eprintln!("username got: {}", username);
        tasks.push(Top5TasksResponse1 {
            task_id: record.task_id,
            author_username: username,
            likes_count: record.likes_count,
            views_count: record.views_count,
        });
    }

    (StatusCode::OK, Json(tasks)).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Top3UsersResponse1 {
    author_username: String,
    likes_count: i64,
}

async fn most_popular_users(State(state): State<Arc<AppState>>) -> Response {
    let url = "http://stat_service:50052";
    let mut client = match StatServiceClient::connect(url).await {
        Ok(client) => client,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    let req = proto::EmptyMessage {};
    eprintln!("req created");
    let request = tonic::Request::new(req);
    eprintln!("request created");
    let response = match client.get_top3_users(request).await {
        Ok(response) => response,
        Err(e) => {
            println!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };
    eprintln!("response got");

    let mut tasks: Vec<Top3UsersResponse1> = vec![];
    for record in response.get_ref().clone().users.iter() {
        let username = match get_username_by_id(&state, record.author_id).await {
            Some(username) => username,
            None => {
                return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
            }
        };
        eprintln!("username got: {}", username);
        tasks.push(Top3UsersResponse1 {
            author_username: username,
            likes_count: record.likes_count,
        });
    }
    (StatusCode::OK, Json(tasks)).into_response()
}
