use notify_rust::{Notification, Timeout};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crate::analytics::{AlertLevel, UsageAnalytics};
use crate::error::{AppError, AppResult};

pub struct NotificationManager {
    last_notifications: HashMap<String, Instant>,
    notification_cooldown: Duration,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            last_notifications: HashMap::new(),
            notification_cooldown: Duration::from_secs(300), // 5 minutes cooldown
        }
    }
    
    pub async fn check_and_send_alerts(&mut self, analytics: &UsageAnalytics, current_balance: u32) {
        // Check balance thresholds
        if current_balance <= 100 {
            self.send_notification_if_needed(
                "critical_balance",
                "Critical Balance Alert",
                &format!("Only {} credits remaining!", current_balance),
                AlertLevel::Critical,
            ).await;
        } else if current_balance <= 500 {
            self.send_notification_if_needed(
                "low_balance",
                "Low Balance Warning",
                &format!("{} credits remaining", current_balance),
                AlertLevel::Warning,
            ).await;
        }
        
        // Check time-based alerts
        if let Some(hours_remaining) = analytics.estimated_hours_remaining {
            if hours_remaining <= 2.0 {
                self.send_notification_if_needed(
                    "time_critical",
                    "Credits Depleting Soon",
                    &format!("Credits will run out in {:.1} hours at current usage rate", hours_remaining),
                    AlertLevel::Critical,
                ).await;
            } else if hours_remaining <= 24.0 {
                self.send_notification_if_needed(
                    "time_warning",
                    "Credits Running Low",
                    &format!("Credits will run out in {:.1} hours at current usage rate", hours_remaining),
                    AlertLevel::Warning,
                ).await;
            }
        }
        
        // Check for unusual usage patterns
        if analytics.usage_rate_per_hour > 0.0 {
            let recent_rate = analytics.usage_rate_per_hour;
            let historical_average = analytics.average_session_usage;
            
            if recent_rate > historical_average * 2.0 {
                self.send_notification_if_needed(
                    "high_usage",
                    "High Usage Detected",
                    &format!("Current usage rate ({:.1}/hour) is significantly higher than average", recent_rate),
                    AlertLevel::Warning,
                ).await;
            }
        }
    }
    
    async fn send_notification_if_needed(
        &mut self,
        notification_id: &str,
        title: &str,
        message: &str,
        level: AlertLevel,
    ) {
        // Check cooldown
        if let Some(last_time) = self.last_notifications.get(notification_id) {
            if last_time.elapsed() < self.notification_cooldown {
                return; // Still in cooldown period
            }
        }
        
        if let Err(e) = self.send_notification(title, message, level).await {
            tracing::error!("Failed to send notification: {}", e);
        } else {
            self.last_notifications.insert(notification_id.to_string(), Instant::now());
        }
    }
    
    pub async fn send_notification(&self, title: &str, message: &str, level: AlertLevel) -> AppResult<()> {
        let mut notification = Notification::new();
        notification
            .summary(title)
            .body(message)
            .appname("orb Credit Monitor")
            .timeout(Timeout::Milliseconds(5000));
        
        // Set icon based on alert level
        match level {
            AlertLevel::Critical => {
                notification.icon("dialog-error");
            }
            AlertLevel::Warning => {
                notification.icon("dialog-warning");
            }
            AlertLevel::Info => {
                notification.icon("dialog-information");
            }
        }
        
        notification.show()
            .map_err(|e| AppError::Notification(format!("Failed to show notification: {}", e)))?;
        
        tracing::info!("Sent notification: {} - {}", title, message);
        Ok(())
    }
    
    pub async fn send_balance_update(&self, current_balance: u32, previous_balance: Option<u32>) -> AppResult<()> {
        if let Some(prev) = previous_balance {
            let change = current_balance as i32 - prev as i32;
            let message = if change < 0 {
                format!("Balance decreased by {} to {} credits", -change, current_balance)
            } else if change > 0 {
                format!("Balance increased by {} to {} credits", change, current_balance)
            } else {
                format!("Balance unchanged at {} credits", current_balance)
            };
            
            self.send_notification("Balance Update", &message, AlertLevel::Info).await
        } else {
            self.send_notification(
                "Balance Update",
                &format!("Current balance: {} credits", current_balance),
                AlertLevel::Info,
            ).await
        }
    }
    
    pub async fn send_error_notification(&self, error_message: &str) -> AppResult<()> {
        self.send_notification(
            "orb Monitor Error",
            &format!("Error: {}", error_message),
            AlertLevel::Warning,
        ).await
    }
    
    pub async fn send_connection_status(&self, is_connected: bool) -> AppResult<()> {
        let (title, message, level) = if is_connected {
            ("Connection Restored", "Successfully reconnected to orb portal", AlertLevel::Info)
        } else {
            ("Connection Lost", "Unable to connect to orb portal", AlertLevel::Warning)
        };
        
        self.send_notification(title, message, level).await
    }
    
    pub fn set_cooldown_duration(&mut self, duration: Duration) {
        self.notification_cooldown = duration;
    }
    
    pub fn clear_notification_history(&mut self) {
        self.last_notifications.clear();
    }
    
    pub async fn test_notifications(&self) -> AppResult<()> {
        // Send test notifications for each level
        self.send_notification(
            "Test Notification - Info",
            "This is a test info notification",
            AlertLevel::Info,
        ).await?;
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        self.send_notification(
            "Test Notification - Warning",
            "This is a test warning notification",
            AlertLevel::Warning,
        ).await?;
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        self.send_notification(
            "Test Notification - Critical",
            "This is a test critical notification",
            AlertLevel::Critical,
        ).await?;
        
        Ok(())
    }
}
