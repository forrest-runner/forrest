use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use hyper::body::Incoming;
use hyper::server::conn::http1::Builder as HttpConnectionBuilder;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use log::trace;
use tokio::net::UnixListener;

use crate::config::Config;
use crate::ingres::WebhookHandler;

struct Handlers {
    webhook: WebhookHandler,
}

pub struct Api {
    listener: UnixListener,
    handlers: Arc<Handlers>,
}

impl Api {
    pub fn new(config: Config, webhook: WebhookHandler) -> std::io::Result<Self> {
        let listener = {
            let cfg = config.get();

            let path = cfg.host.base_dir.join("api.sock");

            let _ = std::fs::remove_file(&path);

            let listener = UnixListener::bind(&path)?;

            // Allow anyone on the system to connect to the socket.
            std::fs::set_permissions(path, Permissions::from_mode(0o777))?;

            listener
        };

        let handlers = Arc::new(Handlers { webhook });

        Ok(Self { listener, handlers })
    }

    pub async fn run(self) -> std::io::Result<()> {
        loop {
            let (sock, _) = self.listener.accept().await?;
            let handlers = self.handlers.clone();

            // Wrap the tokio socket in something that hyper understands.
            let sock = TokioIo::new(sock);

            tokio::task::spawn(async move {
                // Wrap our handler function in something that hyper understands.
                let service = service_fn(|req| api_handler(req, &handlers));

                HttpConnectionBuilder::new()
                    .serve_connection(sock, service)
                    .await
            });
        }
    }
}

async fn api_handler(
    request: Request<Incoming>,
    handlers: &Handlers,
) -> anyhow::Result<Response<String>> {
    let first_path_component = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("");

    trace!("API request for: {}", first_path_component);

    match first_path_component {
        "webhook" => handlers.webhook.handle(request).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("File not found".into())
            .unwrap()),
    }
}
