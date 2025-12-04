use std::sync::Arc;
use chrono::{DateTime, Utc, Timelike};
use serde::{Deserialize, Serialize};
use crate::database::{Database, BalanceRecord, UsageRecord};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalytics {
    pub current_balance: Option<u32>,
    pub usage_rate_per_hour: f64,
    pub usage_rate_per_day: f64,
    pub estimated_days_remaining: Option<f64>,
    pub estimated_hours_remaining: Option<f64>,
    pub total_usage_period: u32,
    pub average_session_usage: f64,
    pub peak_usage_hour: Option<u8>,
    pub trend: UsageTrend,
    pub efficiency_score: f64,
    pub balance_history: Vec<BalanceDataPoint>,
    pub usage_history: Vec<UsageDataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceDataPoint {
    pub timestamp: DateTime<Utc>,
    pub balance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDataPoint {
    pub timestamp: DateTime<Utc>,
    pub usage_amount: u32,
    pub rate_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsageTrend {
    Increasing,
    Decreasing,
    Stable,
    Insufficient,
}

pub struct AnalyticsEngine {
    database: Arc<Database>,
}

impl AnalyticsEngine {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
    
    pub async fn calculate_usage_analytics(&self, hours: u32) -> AppResult<UsageAnalytics> {
        let balance_history = self.database.get_balance_history(hours).await?;
        let usage_history = self.database.get_usage_history(hours).await?;
        
        let current_balance = balance_history.last().map(|b| b.amount);
        
        // Calculate usage rates
        let (usage_rate_per_hour, usage_rate_per_day) = self.calculate_usage_rates(&usage_history)?;
        
        // Calculate time remaining estimates
        let (estimated_hours_remaining, estimated_days_remaining) = 
            self.calculate_time_remaining(current_balance, usage_rate_per_hour);
        
        // Calculate trend
        let trend = self.calculate_trend(&balance_history)?;
        
        // Calculate efficiency metrics
        let efficiency_score = self.calculate_efficiency_score(&usage_history)?;
        let average_session_usage = self.calculate_average_session_usage(&usage_history)?;
        let peak_usage_hour = self.calculate_peak_usage_hour(&usage_history)?;
        
        // Prepare data points for charts
        let balance_data_points = balance_history.iter()
            .map(|record| BalanceDataPoint {
                timestamp: record.timestamp,
                balance: record.amount,
            })
            .collect();
        
        let usage_data_points = usage_history.iter()
            .map(|record| UsageDataPoint {
                timestamp: record.timestamp,
                usage_amount: record.usage_amount,
                rate_per_hour: if record.duration_minutes > 0 {
                    (record.usage_amount as f64 / record.duration_minutes as f64) * 60.0
                } else {
                    0.0
                },
            })
            .collect();
        
        Ok(UsageAnalytics {
            current_balance,
            usage_rate_per_hour,
            usage_rate_per_day,
            estimated_days_remaining,
            estimated_hours_remaining,
            total_usage_period: hours,
            average_session_usage,
            peak_usage_hour,
            trend,
            efficiency_score,
            balance_history: balance_data_points,
            usage_history: usage_data_points,
        })
    }
    
    fn calculate_usage_rates(&self, usage_history: &[UsageRecord]) -> AppResult<(f64, f64)> {
        if usage_history.is_empty() {
            return Ok((0.0, 0.0));
        }
        
        let total_usage: u32 = usage_history.iter().map(|r| r.usage_amount).sum();
        let total_minutes: u32 = usage_history.iter().map(|r| r.duration_minutes).sum();
        
        if total_minutes == 0 {
            return Ok((0.0, 0.0));
        }
        
        let usage_rate_per_minute = total_usage as f64 / total_minutes as f64;
        let usage_rate_per_hour = usage_rate_per_minute * 60.0;
        let usage_rate_per_day = usage_rate_per_hour * 24.0;
        
        Ok((usage_rate_per_hour, usage_rate_per_day))
    }
    
    fn calculate_time_remaining(&self, current_balance: Option<u32>, usage_rate_per_hour: f64) -> (Option<f64>, Option<f64>) {
        if let Some(balance) = current_balance {
            if usage_rate_per_hour > 0.0 {
                let hours_remaining = balance as f64 / usage_rate_per_hour;
                let days_remaining = hours_remaining / 24.0;
                (Some(hours_remaining), Some(days_remaining))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    }
    
    fn calculate_trend(&self, balance_history: &[BalanceRecord]) -> AppResult<UsageTrend> {
        if balance_history.len() < 3 {
            return Ok(UsageTrend::Insufficient);
        }
        
        // Calculate the slope of balance over time using linear regression
        let n = balance_history.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;
        
        for (i, record) in balance_history.iter().enumerate() {
            let x = i as f64;
            let y = record.amount as f64;
            
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        
        // Classify trend based on slope
        if slope < -1.0 {
            Ok(UsageTrend::Increasing) // Balance decreasing = usage increasing
        } else if slope > 1.0 {
            Ok(UsageTrend::Decreasing) // Balance increasing = usage decreasing
        } else {
            Ok(UsageTrend::Stable)
        }
    }
    
    fn calculate_efficiency_score(&self, usage_history: &[UsageRecord]) -> AppResult<f64> {
        if usage_history.is_empty() {
            return Ok(0.0);
        }
        
        // Calculate efficiency based on consistency of usage patterns
        let usage_rates: Vec<f64> = usage_history.iter()
            .filter(|r| r.duration_minutes > 0)
            .map(|r| r.usage_amount as f64 / r.duration_minutes as f64)
            .collect();
        
        if usage_rates.is_empty() {
            return Ok(0.0);
        }
        
        let mean = usage_rates.iter().sum::<f64>() / usage_rates.len() as f64;
        let variance = usage_rates.iter()
            .map(|rate| (rate - mean).powi(2))
            .sum::<f64>() / usage_rates.len() as f64;
        
        let coefficient_of_variation = if mean > 0.0 {
            variance.sqrt() / mean
        } else {
            1.0
        };
        
        // Convert to efficiency score (lower variation = higher efficiency)
        let efficiency = (1.0 - coefficient_of_variation.min(1.0)).max(0.0) * 100.0;
        
        Ok(efficiency)
    }
    
    fn calculate_average_session_usage(&self, usage_history: &[UsageRecord]) -> AppResult<f64> {
        if usage_history.is_empty() {
            return Ok(0.0);
        }
        
        let total_usage: u32 = usage_history.iter().map(|r| r.usage_amount).sum();
        let average = total_usage as f64 / usage_history.len() as f64;
        
        Ok(average)
    }
    
    fn calculate_peak_usage_hour(&self, usage_history: &[UsageRecord]) -> AppResult<Option<u8>> {
        if usage_history.is_empty() {
            return Ok(None);
        }
        
        let mut hourly_usage = [0u32; 24];
        
        for record in usage_history {
            let hour = record.timestamp.hour() as usize;
            if hour < 24 {
                hourly_usage[hour] += record.usage_amount;
            }
        }
        
        let peak_hour = hourly_usage.iter()
            .enumerate()
            .max_by_key(|(_, &usage)| usage)
            .map(|(hour, _)| hour as u8);
        
        Ok(peak_hour)
    }
    
    pub async fn get_usage_prediction(&self, hours_ahead: u32) -> AppResult<f64> {
        let analytics = self.calculate_usage_analytics(24).await?;
        
        let predicted_usage = analytics.usage_rate_per_hour * hours_ahead as f64;
        Ok(predicted_usage)
    }
    
    pub async fn get_balance_alerts(&self, low_threshold: u32, critical_threshold: u32) -> AppResult<Vec<AlertInfo>> {
        let analytics = self.calculate_usage_analytics(24).await?;
        let mut alerts = Vec::new();
        
        if let Some(current_balance) = analytics.current_balance {
            if current_balance <= critical_threshold {
                alerts.push(AlertInfo {
                    level: AlertLevel::Critical,
                    message: format!("Critical: Only {} credits remaining!", current_balance),
                    estimated_time_remaining: analytics.estimated_hours_remaining,
                });
            } else if current_balance <= low_threshold {
                alerts.push(AlertInfo {
                    level: AlertLevel::Warning,
                    message: format!("Warning: {} credits remaining", current_balance),
                    estimated_time_remaining: analytics.estimated_hours_remaining,
                });
            }
            
            // Check if balance will run out soon based on current usage
            if let Some(hours_remaining) = analytics.estimated_hours_remaining {
                if hours_remaining <= 2.0 {
                    alerts.push(AlertInfo {
                        level: AlertLevel::Critical,
                        message: format!("Credits will be depleted in {:.1} hours at current usage rate", hours_remaining),
                        estimated_time_remaining: Some(hours_remaining),
                    });
                } else if hours_remaining <= 24.0 {
                    alerts.push(AlertInfo {
                        level: AlertLevel::Warning,
                        message: format!("Credits will be depleted in {:.1} hours at current usage rate", hours_remaining),
                        estimated_time_remaining: Some(hours_remaining),
                    });
                }
            }
        }
        
        Ok(alerts)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertInfo {
    pub level: AlertLevel,
    pub message: String,
    pub estimated_time_remaining: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}
