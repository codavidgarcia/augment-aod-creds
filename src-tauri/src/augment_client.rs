use reqwest::{Client, header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT}};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::error::{AppError, AppResult};

const AUGMENT_BASE_URL: &str = "https://app.augmentcode.com";

/// Response from /api/credits endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditsResponse {
    pub usage_units_available: i64,
    pub usage_units_used_this_billing_cycle: i64,
    pub usage_units_remaining: i64,
    pub usage_units_consumed_this_billing_cycle: i64,
}

/// Response from /api/subscription endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionResponse {
    pub portal_url: Option<String>,
    pub plan_id: String,
    pub plan_type: i32,
    pub plan_name: String,
    pub billing_period_end: String,
    pub trial_period_end: Option<String>,
    pub credit_consumption_min_date: Option<String>,
    pub credits_renewing_each_billing_cycle: i64,
    pub credits_included_this_billing_cycle: i64,
    pub billing_cycle_billing_amount: String,
    pub monthly_total_cost: String,
    pub price_per_seat: String,
    pub max_num_seats: i32,
    pub number_of_seats_this_billing_cycle: i32,
    pub number_of_seats_next_billing_cycle: i32,
    pub subscription_end_date: Option<String>,
    pub plan_is_expired: bool,
    pub auto_top_up_available: bool,
    pub teams_allowed: bool,
    pub additional_usage_unit_cost: String,
    pub scheduled_target_plan_id: Option<String>,
    pub usage_unit_display_name: String,
    pub usage_units_per_seat: i64,
    pub plan_facts: Vec<String>,
    pub trial_grant: i64,
    pub cancelled_due_to_payment_failure: bool,
    pub is_cancellation_immediate: bool,
    pub next_billing_cycle_plan_name: String,
}

/// Response from /api/user endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub email: String,
    pub is_admin: bool,
    pub is_self_serve_team_member: bool,
    pub tenant_tier: String,
    pub is_subscription_pending: bool,
    pub show_team_management_link: bool,
    pub user_source_submitted: bool,
    pub business_email_verified: bool,
    pub preferred_team_name: String,
}

/// Response from /api/credit-analytics-info endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditAnalyticsInfoResponse {
    pub total_credits_consumed: String,
    #[serde(default)]
    pub credits_percent_increase_over_previous_period: Option<f64>,
    #[serde(default)]
    pub active_user_count: Option<i32>,
    #[serde(default)]
    pub users_percent_increase_over_previous_period: Option<f64>,
}

/// Response from /api/credit-consumption endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditConsumptionResponse {
    pub data_points: Vec<ConsumptionDataPoint>,
}

/// A single data point in consumption response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumptionDataPoint {
    pub date_range: DateRange,
    #[serde(default)]
    pub credits_consumed: Option<String>,
    #[serde(default)]
    pub group_key: Option<String>,
}

/// Date range for a data point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    pub start_date_iso: String,
    pub end_date_iso: String,
}

/// Aggregated daily usage for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    pub total_credits: i64,
}

/// Usage by model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model_name: String,
    pub credits: i64,
}

/// Usage by activity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityUsage {
    pub activity_type: String,
    pub credits: i64,
}

/// Combined balance info for the app (legacy, not used)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentBalanceInfo {
    pub credits_remaining: i64,
    pub credits_used: i64,
    pub credits_total: i64,
    pub plan_name: String,
    pub billing_period_end: String,
    pub user_email: String,
}

/// Client for Augment API
pub struct AugmentClient {
    client: Client,
    session_cookie: String,
}

impl AugmentClient {
    pub fn new(session_cookie: String) -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            session_cookie,
        })
    }

    fn build_headers(&self) -> AppResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        );
        
        let cookie_value = format!("_session={}", self.session_cookie);
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&cookie_value)
                .map_err(|e| AppError::Unknown(format!("Invalid cookie header: {}", e)))?
        );

        Ok(headers)
    }

    /// Fetch current credits balance
    pub async fn fetch_credits(&self) -> AppResult<CreditsResponse> {
        let url = format!("{}/api/credits", AUGMENT_BASE_URL);
        tracing::info!("🔄 Fetching credits from: {}", url);

        let response = self.client
            .get(&url)
            .headers(self.build_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("❌ Credits API error: {} - {}", status, body);
            return Err(AppError::Auth(format!("API error: {} - Session may have expired", status)));
        }

        let credits: CreditsResponse = response.json().await?;
        tracing::info!("✅ Credits fetched: {} remaining", credits.usage_units_remaining);
        Ok(credits)
    }

    /// Fetch subscription info
    pub async fn fetch_subscription(&self) -> AppResult<SubscriptionResponse> {
        let url = format!("{}/api/subscription", AUGMENT_BASE_URL);
        tracing::info!("🔄 Fetching subscription from: {}", url);

        let response = self.client
            .get(&url)
            .headers(self.build_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Auth(format!("Subscription API error: {}", status)));
        }

        let subscription: SubscriptionResponse = response.json().await?;
        tracing::info!("✅ Subscription fetched: {}", subscription.plan_name);
        Ok(subscription)
    }

    /// Fetch user info
    pub async fn fetch_user(&self) -> AppResult<UserResponse> {
        let url = format!("{}/api/user", AUGMENT_BASE_URL);
        tracing::info!("🔄 Fetching user from: {}", url);

        let response = self.client
            .get(&url)
            .headers(self.build_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Auth(format!("User API error: {}", status)));
        }

        let user: UserResponse = response.json().await?;
        tracing::info!("✅ User fetched: {}", user.email);
        Ok(user)
    }

    /// Fetch complete balance info (combines credits + subscription + user)
    pub async fn fetch_balance_info(&self) -> AppResult<AugmentBalanceInfo> {
        // Fetch all data in parallel
        let (credits, subscription, user) = tokio::try_join!(
            self.fetch_credits(),
            self.fetch_subscription(),
            self.fetch_user()
        )?;

        let total_credits = subscription.credits_included_this_billing_cycle + subscription.trial_grant;

        Ok(AugmentBalanceInfo {
            credits_remaining: credits.usage_units_remaining,
            credits_used: credits.usage_units_consumed_this_billing_cycle,
            credits_total: total_credits,
            plan_name: subscription.plan_name,
            billing_period_end: subscription.billing_period_end,
            user_email: user.email,
        })
    }

    /// Fetch credit analytics info
    pub async fn fetch_credit_analytics_info(&self, days: u32) -> AppResult<CreditAnalyticsInfoResponse> {
        let end_date = chrono::Utc::now();
        let start_date = end_date - chrono::Duration::days(days as i64);

        let start_iso = start_date.format("%Y-%m-%dT00:00:00.000Z").to_string();
        let end_iso = end_date.format("%Y-%m-%dT00:00:00.000Z").to_string();

        let url = format!(
            "{}/api/credit-analytics-info?startDateIso={}&endDateIso={}",
            AUGMENT_BASE_URL,
            urlencoding::encode(&start_iso),
            urlencoding::encode(&end_iso)
        );
        tracing::info!("🔄 Fetching credit analytics info from: {}", url);

        let response = self.client
            .get(&url)
            .headers(self.build_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Auth(format!("Credit analytics API error: {}", status)));
        }

        let analytics: CreditAnalyticsInfoResponse = response.json().await?;
        tracing::info!("✅ Credit analytics fetched: {} total consumed", analytics.total_credits_consumed);
        Ok(analytics)
    }

    /// Fetch daily credit consumption (groupBy=NONE, granularity=DAY)
    pub async fn fetch_daily_consumption(&self, days: u32) -> AppResult<CreditConsumptionResponse> {
        let end_date = chrono::Utc::now();
        let start_date = end_date - chrono::Duration::days(days as i64);

        let start_iso = start_date.format("%Y-%m-%dT00:00:00.000Z").to_string();
        let end_iso = end_date.format("%Y-%m-%dT00:00:00.000Z").to_string();

        let url = format!(
            "{}/api/credit-consumption?groupBy=NONE&granularity=DAY&startDateIso={}&endDateIso={}",
            AUGMENT_BASE_URL,
            urlencoding::encode(&start_iso),
            urlencoding::encode(&end_iso)
        );
        tracing::info!("🔄 Fetching daily consumption from: {}", url);

        let response = self.client
            .get(&url)
            .headers(self.build_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Auth(format!("Daily consumption API error: {}", status)));
        }

        let consumption: CreditConsumptionResponse = response.json().await?;
        tracing::info!("✅ Daily consumption fetched: {} data points", consumption.data_points.len());
        Ok(consumption)
    }

    /// Fetch consumption by model (groupBy=MODEL_NAME, granularity=TOTAL)
    pub async fn fetch_consumption_by_model(&self, days: u32) -> AppResult<CreditConsumptionResponse> {
        let end_date = chrono::Utc::now();
        let start_date = end_date - chrono::Duration::days(days as i64);

        let start_iso = start_date.format("%Y-%m-%dT00:00:00.000Z").to_string();
        let end_iso = end_date.format("%Y-%m-%dT00:00:00.000Z").to_string();

        let url = format!(
            "{}/api/credit-consumption?groupBy=MODEL_NAME&granularity=TOTAL&startDateIso={}&endDateIso={}",
            AUGMENT_BASE_URL,
            urlencoding::encode(&start_iso),
            urlencoding::encode(&end_iso)
        );
        tracing::info!("🔄 Fetching consumption by model from: {}", url);

        let response = self.client
            .get(&url)
            .headers(self.build_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Auth(format!("Model consumption API error: {}", status)));
        }

        let consumption: CreditConsumptionResponse = response.json().await?;
        tracing::info!("✅ Model consumption fetched: {} models", consumption.data_points.len());
        Ok(consumption)
    }

    /// Fetch consumption by activity type (groupBy=ACTIVITY_TYPE, granularity=TOTAL)
    pub async fn fetch_consumption_by_activity(&self, days: u32) -> AppResult<CreditConsumptionResponse> {
        let end_date = chrono::Utc::now();
        let start_date = end_date - chrono::Duration::days(days as i64);

        let start_iso = start_date.format("%Y-%m-%dT00:00:00.000Z").to_string();
        let end_iso = end_date.format("%Y-%m-%dT00:00:00.000Z").to_string();

        let url = format!(
            "{}/api/credit-consumption?groupBy=ACTIVITY_TYPE&granularity=TOTAL&startDateIso={}&endDateIso={}",
            AUGMENT_BASE_URL,
            urlencoding::encode(&start_iso),
            urlencoding::encode(&end_iso)
        );
        tracing::info!("🔄 Fetching consumption by activity from: {}", url);

        let response = self.client
            .get(&url)
            .headers(self.build_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Auth(format!("Activity consumption API error: {}", status)));
        }

        let consumption: CreditConsumptionResponse = response.json().await?;
        tracing::info!("✅ Activity consumption fetched: {} types", consumption.data_points.len());
        Ok(consumption)
    }

    /// Convert consumption response to daily usage list
    pub fn to_daily_usage(&self, consumption: &CreditConsumptionResponse) -> Vec<DailyUsage> {
        consumption.data_points.iter()
            .filter_map(|dp| {
                let credits = dp.credits_consumed.as_ref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);

                if credits > 0 {
                    // Extract date from start_date_iso (e.g., "2025-11-06T00:00:00Z" -> "2025-11-06")
                    let date = dp.date_range.start_date_iso.split('T').next()
                        .unwrap_or(&dp.date_range.start_date_iso)
                        .to_string();
                    Some(DailyUsage { date, total_credits: credits })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Convert consumption response to model usage list
    pub fn to_model_usage(&self, consumption: &CreditConsumptionResponse) -> Vec<ModelUsage> {
        consumption.data_points.iter()
            .filter_map(|dp| {
                let credits = dp.credits_consumed.as_ref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);

                if credits > 0 {
                    dp.group_key.as_ref().map(|model| ModelUsage {
                        model_name: model.clone(),
                        credits,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Convert consumption response to activity usage list
    pub fn to_activity_usage(&self, consumption: &CreditConsumptionResponse) -> Vec<ActivityUsage> {
        consumption.data_points.iter()
            .filter_map(|dp| {
                let credits = dp.credits_consumed.as_ref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);

                if credits > 0 {
                    dp.group_key.as_ref().map(|activity| ActivityUsage {
                        activity_type: activity.clone(),
                        credits,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Validate session cookie by testing the API
    pub async fn validate_session(&self) -> AppResult<bool> {
        match self.fetch_user().await {
            Ok(_) => Ok(true),
            Err(AppError::Auth(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

