use dotenv::dotenv;
use main_service;
use std::env;

#[tokio::main]
async fn main() {
    match std::env::var("DATABASE_URL") {
        Ok(_) => {}
        Err(_) => {
            dotenv().ok();
        }
    };

    match std::env::var("DATABASE_URL") {
        Ok(url) => {
            println!("DATABASE_URL: {}", url);
        }
        Err(_) => {
            std::process::exit(1);
        }
    };

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ./main_service [port]");
        std::process::exit(1);
    }
    let port = &args[1];
    let host = format!("0.0.0.0:{}", port);

    let db_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("DATABASE_URL not set");
            std::process::exit(1);
        }
    };
    let app = main_service::create_app(&db_url, false).await;

    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
