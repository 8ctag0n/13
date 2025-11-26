use anyhow::Result;
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

/// Test backend server helper
/// Starts a witness-storage server on a random free port
#[allow(dead_code)]
pub struct TestBackend {
    port: u16,
    handle: Option<JoinHandle<()>>,
}

#[allow(dead_code)]
impl TestBackend {
    /// Start a new test backend server on a random free port
    pub async fn start() -> Result<Self> {
        // Find a free port
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener); // Release the port

        // Start the server in a background task
        let handle = tokio::spawn(async move {
            // Create storage and app state
            let storage = witness_storage::WitnessStorage::new();
            let app_state = witness_storage::AppState {
                storage: Arc::new(RwLock::new(storage)),
            };

            // Create the router
            let app = witness_storage::create_router(app_state);

            // Bind to the port
            let listener = tokio::net::TcpListener::bind(
                format!("127.0.0.1:{}", port)
            )
            .await
            .expect("Failed to bind test server");

            // Serve
            axum::serve(listener, app)
                .await
                .expect("Test server failed");
        });

        // Wait a bit for the server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(Self {
            port,
            handle: Some(handle),
        })
    }

    /// Get the base URL for this backend
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Shutdown the backend server
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

impl Drop for TestBackend {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}
