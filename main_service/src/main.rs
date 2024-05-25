use main_service;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ./main_service [port]");
        std::process::exit(1);
    }
    let port = &args[1];
    let host = format!("0.0.0.0:{}", port);

    let db_url = "postgresql://postgres:qwerty@localhost:5432/main_service_soa";

    let app = main_service::create_app(db_url, false).await;

    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
