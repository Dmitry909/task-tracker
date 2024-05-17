use main_service;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ./auth_service [port]");
        std::process::exit(1);
    }
    let port = &args[1];
    let host = format!("0.0.0.0:{}", port);

    let users_db_url = "postgresql://postgres:qwerty@localhost:5432/auth_service";

    let app = main_service::create_app(users_db_url, false).await;

    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
