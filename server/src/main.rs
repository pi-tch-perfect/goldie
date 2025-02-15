mod router;
mod routes {
    pub mod admin;
    pub mod healthcheck;
    pub mod karaoke;
    pub mod streaming;
    pub mod sys;
}
mod state;
mod actors {
    pub mod song_coordinator;
    pub mod video_downloader;
}

mod lib {
    pub mod file_storage;
    pub mod os;
    pub mod pitch_shifter;
    pub mod video_extractor;
    pub mod yt_downloader;
    pub mod xml_mpd;
    pub mod test;
}

use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use crate::router::create_router_with_state;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("server=error,tower_http=warn"))
                .unwrap(),
        )
        .init();

    let cors_layer = CorsLayer::new()
        .allow_origin(Any) // Allows all origins
        .allow_methods(Any) // Allows all HTTP methods
        .allow_headers(Any); // Allows all headers

    let app = create_router_with_state()
        .await
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http());

    println!("Server started. Please listen on 127.0.0.1:8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
