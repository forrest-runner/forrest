use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::server::conn::http1::Builder as HttpConnectionBuilder;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use log::{error, info, trace};
use octocrab::models::webhook_events::EventInstallation;
use octocrab::models::webhook_events::{WebhookEvent, WebhookEventPayload};
use octocrab::models::workflows::Job;
use sha2::Sha256;
use tokio::net::UnixListener;

use crate::auth::Auth;
use crate::config::{Config, ConfigFile};
use crate::jobs::Manager as JobManager;
use crate::machines::OwnerAndRepo;

pub struct WebhookHandler {
    config: Config,
    auth: Arc<Auth>,
    job_manager: JobManager,
    listener: UnixListener,
}

impl WebhookHandler {
    pub fn new(config: Config, auth: Arc<Auth>, job_manager: JobManager) -> std::io::Result<Self> {
        let listener = {
            let cfg = config.get();

            let path = cfg.host.base_dir.join("webhook.sock");

            let _ = std::fs::remove_file(&path);

            let listener = UnixListener::bind(&path)?;

            // Allow anyone on the system to connect to the socket.
            std::fs::set_permissions(path, Permissions::from_mode(0o777))?;

            listener
        };

        Ok(Self {
            config,
            auth,
            job_manager,
            listener,
        })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        loop {
            let (sock, _) = self.listener.accept().await?;
            let config = self.config.get();
            let auth = self.auth.clone();
            let job_manager = self.job_manager.clone();

            // Wrap the tokio socket in something that hyper understands.
            let sock = TokioIo::new(sock);

            tokio::task::spawn(async move {
                // Wrap our handler function in something that hyper understands.
                let service =
                    service_fn(|conn| webhook_handler(conn, &config, &auth, &job_manager));

                HttpConnectionBuilder::new()
                    .serve_connection(sock, service)
                    .await
            });
        }
    }
}

async fn webhook_handler(
    request: Request<Incoming>,
    config: &ConfigFile,
    auth: &Auth,
    job_manager: &JobManager,
) -> anyhow::Result<Response<String>> {
    let (parts, body) = request.into_parts();

    if parts.uri.path() != "/webhook" {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not found".into())
            .unwrap());
    }

    if parts.method != Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body("Only HTTP POST is allowed".into())
            .unwrap());
    }

    let event_type = match parts.headers.get("X-GitHub-Event") {
        Some(et) => et,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Request is missing an X-GitHub-Event Header".into())
                .unwrap());
        }
    };

    let event_type = match event_type.to_str() {
        Ok(et) => et,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to decode X-GitHub-Event Header".into())
                .unwrap());
        }
    };

    let signature = match parts.headers.get("X-Hub-Signature-256") {
        Some(sig) => sig,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Request is missing an X-Hub-Signature-256 Header".into())
                .unwrap());
        }
    };

    let signature = signature
        .to_str()
        .ok()
        .and_then(|sig| sig.strip_prefix("sha256="))
        .and_then(|sig| hex::decode(sig).ok());

    let signature = match signature {
        Some(sig) => sig,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to decode X-Hub-Signature-256 Header".into())
                .unwrap());
        }
    };

    let secret = config.github.webhook_secret.as_bytes();

    let content = {
        let content = body.collect().await?.to_bytes();

        let mut hmac: Hmac<Sha256> = Hmac::new_from_slice(secret).unwrap();
        hmac.update(&content);
        let content_valid = hmac.verify_slice(&signature);

        if content_valid.is_err() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Signature validation failed".into())
                .unwrap());
        }

        content
    };

    trace!("Got webhook event of type {event_type}");

    let event = match WebhookEvent::try_from_header_and_body(event_type, &content) {
        Ok(ev) => ev,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to parse request body".into())
                .unwrap());
        }
    };

    let job = match event.specific {
        WebhookEventPayload::WorkflowJob(job) => job,
        _ => {
            return Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body("".into())
                .unwrap())
        }
    };

    let oar = {
        let repository = match event.repository {
            Some(repo) => repo,
            None => {
                error!("Got workflow_job webhook event without repository field");
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("Workflow job is missing a repository field".into())
                    .unwrap());
            }
        };

        let owner = match repository.owner {
            Some(owner) => owner.login,
            None => {
                error!("Got workflow_job webhook event without user in repository field");
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("Workflow job repository is missing an owner field".into())
                    .unwrap());
            }
        };

        OwnerAndRepo::new(owner, repository.name)
    };

    let exists = config
        .repositories
        .get(oar.owner())
        .and_then(|repos| repos.get(oar.repository()))
        .is_some();

    if !exists {
        info!("Refusing to service webhook from unlisted user/repo {oar}");
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Unauthorized user/repo combination".into())
            .unwrap());
    }

    let installation_id = match event.installation {
        Some(EventInstallation::Full(inst)) => inst.id,
        Some(EventInstallation::Minimal(inst)) => inst.id,
        None => {
            error!("Got webhook event that was not sent by an installation");
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("The webhook event is missing an installation id".into())
                .unwrap());
        }
    };

    let workflow_job: Job = match serde_json::from_value(job.workflow_job) {
        Ok(workflow_job) => workflow_job,
        Err(err) => {
            error!("Could not parse workflow job received from webhook: {err}");
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to parse workflow job".into())
                .unwrap());
        }
    };

    info!(
        "Got webhook event for {oar} with labels: {}",
        workflow_job.labels.join(",")
    );

    // Associate the user with their installation id so we can make API
    // requests on their behalf later.
    auth.update_user(oar.owner(), installation_id);

    if let Some(triplet) = oar.into_triplet_via_labels(&workflow_job.labels) {
        job_manager.status_feedback(
            &triplet,
            workflow_job.id,
            workflow_job.run_id,
            workflow_job.status,
            workflow_job.runner_name.as_deref(),
        );
    }

    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body("".into())
        .unwrap())
}
