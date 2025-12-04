use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // Legacy Orb fields (kept for backward compatibility)
    #[serde(default)]
    pub orb_token: Option<String>,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub pricing_unit_id: Option<String>,

    // New Augment API authentication
    #[serde(default)]
    pub session_cookie: Option<String>,
    #[serde(default)]
    pub user_email: Option<String>,

    // App settings
    pub polling_interval_seconds: u64,
    pub low_balance_threshold: u32,
    pub critical_balance_threshold: u32,
    pub enable_notifications: bool,
    pub enable_sound_alerts: bool,
    pub auto_start: bool,
    pub window_always_on_top: bool,
    pub compact_mode: bool,
    pub theme: Theme,
    pub data_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            // Legacy Orb fields
            orb_token: None,
            customer_id: None,
            pricing_unit_id: None,
            // New Augment API fields
            session_cookie: None,
            user_email: None,
            // App settings
            polling_interval_seconds: 60,
            low_balance_threshold: 500,
            critical_balance_threshold: 100,
            enable_notifications: true,
            enable_sound_alerts: false,
            auto_start: false,
            window_always_on_top: false,
            compact_mode: true,
            theme: Theme::System,
            data_retention_days: 30,
        }
    }
}

impl AppConfig {
    pub async fn load() -> AppResult<Self> {
        let config_path = Self::config_file_path()?;

        if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path).await?;
            let config: AppConfig = serde_json::from_str(&content)?;

            // Keyring disabled for development - token is loaded from config file
            // if config.orb_token.is_none() {
            //     if let Ok(entry) = keyring::Entry::new("augment-credit-monitor", "orb-token") {
            //         if let Ok(token) = entry.get_password() {
            //             config.orb_token = Some(token);
            //         }
            //     }
            // }

            Ok(config)
        } else {
            let config = Self::default();
            config.save().await?;
            Ok(config)
        }
    }
    
    pub async fn save(&self) -> AppResult<()> {
        let config_path = Self::config_file_path()?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // For now, save everything to file (including token) for simplicity
        // In production, you'd want to use keychain for the token
        let content = serde_json::to_string_pretty(&self)?;
        tokio::fs::write(&config_path, content).await?;

        // Keyring disabled for development - token is saved in config file
        // if let Some(token) = &self.orb_token {
        //     if let Ok(entry) = keyring::Entry::new("orb-credit-monitor", "orb-token") {
        //         let _ = entry.set_password(token); // Ignore keyring errors for now
        //     }
        // }

        tracing::info!("Configuration saved successfully to {:?}", config_path);
        Ok(())
    }
    
    pub fn validate(&self) -> AppResult<()> {
        if self.polling_interval_seconds < 30 {
            return Err(AppError::Config(
                config::ConfigError::Message("Polling interval must be at least 30 seconds".to_string())
            ));
        }
        
        if self.critical_balance_threshold >= self.low_balance_threshold {
            return Err(AppError::Config(
                config::ConfigError::Message("Critical threshold must be less than low threshold".to_string())
            ));
        }
        
        if self.data_retention_days == 0 {
            return Err(AppError::Config(
                config::ConfigError::Message("Data retention must be at least 1 day".to_string())
            ));
        }
        
        Ok(())
    }
    
    fn config_file_path() -> AppResult<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AppError::Config(
                config::ConfigError::Message("Could not find config directory".to_string())
            ))?;
        
        Ok(config_dir.join("orb-credit-monitor").join("config.json"))
    }



    /// Check if legacy Orb token is configured
    pub fn is_token_configured(&self) -> bool {
        self.orb_token.is_some() && !self.orb_token.as_ref().unwrap().is_empty()
    }

    /// Check if legacy Orb is fully configured
    pub fn is_orb_configured(&self) -> bool {
        self.orb_token.is_some() &&
        self.customer_id.is_some() &&
        self.pricing_unit_id.is_some() &&
        !self.orb_token.as_ref().unwrap().is_empty() &&
        !self.customer_id.as_ref().unwrap().is_empty() &&
        !self.pricing_unit_id.as_ref().unwrap().is_empty()
    }

    /// Check if new Augment session is configured
    pub fn is_augment_configured(&self) -> bool {
        self.session_cookie.is_some() &&
        !self.session_cookie.as_ref().unwrap().is_empty()
    }

    /// Check if any authentication method is configured
    pub fn is_authenticated(&self) -> bool {
        self.is_augment_configured() || self.is_orb_configured()
    }

    /// Set the Augment session cookie
    pub fn set_session_cookie(&mut self, cookie: String, email: Option<String>) {
        self.session_cookie = Some(cookie);
        self.user_email = email;
    }

    /// Clear the Augment session
    pub fn clear_augment_session(&mut self) {
        self.session_cookie = None;
        self.user_email = None;
    }

    pub fn parse_orb_url(&mut self, url: &str) -> AppResult<()> {
        use url::Url;

        let parsed_url = Url::parse(url)
            .map_err(|_| AppError::Config(
                config::ConfigError::Message("Invalid URL format".to_string())
            ))?;

        // Validate it's a withorb.com URL
        if parsed_url.host_str() != Some("portal.withorb.com") {
            return Err(AppError::Config(
                config::ConfigError::Message("URL must be from portal.withorb.com".to_string())
            ));
        }

        // Extract customer_id from path: /api/v1/customers/{customer_id}/ledger_summary
        let path_segments: Vec<&str> = parsed_url.path_segments()
            .ok_or_else(|| AppError::Config(
                config::ConfigError::Message("Invalid URL path".to_string())
            ))?
            .collect();

        if path_segments.len() < 5 ||
           path_segments[0] != "api" ||
           path_segments[1] != "v1" ||
           path_segments[2] != "customers" ||
           path_segments[4] != "ledger_summary" {
            return Err(AppError::Config(
                config::ConfigError::Message("URL must be in format: /api/v1/customers/{customer_id}/ledger_summary".to_string())
            ));
        }

        let customer_id = path_segments[3].to_string();

        // Extract query parameters
        let mut pricing_unit_id = None;
        let mut token = None;

        for (key, value) in parsed_url.query_pairs() {
            match key.as_ref() {
                "pricing_unit_id" => pricing_unit_id = Some(value.to_string()),
                "token" => token = Some(value.to_string()),
                _ => {} // Ignore other parameters
            }
        }

        let pricing_unit_id = pricing_unit_id.ok_or_else(|| AppError::Config(
            config::ConfigError::Message("URL must contain pricing_unit_id parameter".to_string())
        ))?;

        let token = token.ok_or_else(|| AppError::Config(
            config::ConfigError::Message("URL must contain token parameter".to_string())
        ))?;

        // Validate extracted values are not empty
        if customer_id.is_empty() || pricing_unit_id.is_empty() || token.is_empty() {
            return Err(AppError::Config(
                config::ConfigError::Message("Extracted values cannot be empty".to_string())
            ));
        }

        // Update configuration
        self.customer_id = Some(customer_id);
        self.pricing_unit_id = Some(pricing_unit_id);
        self.orb_token = Some(token);

        Ok(())
    }

    pub fn build_ledger_url(&self) -> AppResult<String> {
        let customer_id = self.customer_id.as_ref()
            .ok_or_else(|| AppError::Config(
                config::ConfigError::Message("Customer ID not configured".to_string())
            ))?;

        let pricing_unit_id = self.pricing_unit_id.as_ref()
            .ok_or_else(|| AppError::Config(
                config::ConfigError::Message("Pricing unit ID not configured".to_string())
            ))?;

        let token = self.orb_token.as_ref()
            .ok_or_else(|| AppError::Config(
                config::ConfigError::Message("Token not configured".to_string())
            ))?;

        Ok(format!(
            "https://portal.withorb.com/api/v1/customers/{}/ledger_summary?pricing_unit_id={}&token={}",
            customer_id, pricing_unit_id, token
        ))
    }
    
    pub fn get_polling_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.polling_interval_seconds)
    }
}
