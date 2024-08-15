use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use octocrab::models::InstallationId;
use octocrab::Octocrab;

use crate::config::Config;

pub struct Auth {
    app: Arc<Octocrab>,
    users: Mutex<HashMap<String, (InstallationId, Arc<Octocrab>)>>,
}

impl Auth {
    pub fn new(config: &Config) -> anyhow::Result<Arc<Self>> {
        let cfg = config.get();

        let app_id = octocrab::models::AppId(cfg.github.app_id);
        let token = {
            let pem = std::fs::read(&cfg.github.jwt_key_file)?;
            jsonwebtoken::EncodingKey::from_rsa_pem(&pem)?
        };

        let app = Arc::new(octocrab::Octocrab::builder().app(app_id, token).build()?);

        let users = Mutex::new(HashMap::new());

        let auth = Self { app, users };

        Ok(Arc::new(auth))
    }

    /// Get an Octocrab instance authenticated as our GitHub application
    pub fn app(&self) -> Arc<Octocrab> {
        self.app.clone()
    }

    /// Create or update a GitHub installation id to user name mapping
    ///
    /// This has to be called at least once before the `user()` method can
    /// be used to log in as a specific user.
    pub fn update_user(&self, user: &str, id: InstallationId) {
        let mut users = self.users.lock().unwrap();

        let is_up_to_date = users
            .get(user)
            .map(|(stored_id, _)| *stored_id == id)
            .unwrap_or(false);

        if !is_up_to_date {
            let oc = Arc::new(self.app.installation(id));
            users.insert(user.to_string(), (id, oc));
        }
    }

    /// Get an Octocrab instance authenticated as `user`
    ///
    /// For this to work the installation ID of said user has to be set first
    /// via the `update_user()` method.
    pub fn user(&self, user: &str) -> Option<Arc<Octocrab>> {
        self.users
            .lock()
            .unwrap()
            .get(user)
            .map(|(_, user)| user.clone())
    }
}
