use std::sync::Arc;
use tokio::sync::RwLock;
use witness_storage::{create_router, AppState, WitnessStorage};

#[tokio::main]
async fn main() {
    env_logger::init();

    let storage = Arc::new(RwLock::new(WitnessStorage::new()));
    let state = AppState { storage };

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();

    println!("Witness storage server listening on http://0.0.0.0:3030");

    axum::serve(listener, app).await.unwrap();
}
