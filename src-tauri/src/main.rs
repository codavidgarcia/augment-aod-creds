// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, Emitter, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState}, WindowEvent, WebviewUrl, WebviewWindowBuilder};
use tauri::webview::PageLoadEvent;
use std::sync::Arc;
use tokio::sync::Mutex;



mod config;
mod database;
mod scraper;
mod analytics;
mod notifications;
mod error;
mod augment_client;

use config::AppConfig;
use database::Database;
use scraper::orbScraper;
use analytics::AnalyticsEngine;
use notifications::NotificationManager;
use error::{AppResult, AppError};
use augment_client::{AugmentClient, CreditsResponse, SubscriptionResponse, AugmentBalanceInfo};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub database: Arc<Database>,
    pub scraper: Arc<orbScraper>,
    pub analytics: Arc<AnalyticsEngine>,
    pub notifications: Arc<Mutex<NotificationManager>>,
    pub window_visible: Arc<Mutex<bool>>,
}

#[tauri::command]
async fn test_connection() -> AppResult<String> {
    Ok("Connection successful".to_string())
}

#[tauri::command]
async fn get_current_balance(state: tauri::State<'_, AppState>) -> AppResult<Option<u32>> {
    let balance = state.database.get_latest_balance().await?;
    Ok(balance.map(|b| b.amount))
}

#[tauri::command]
async fn fetch_fresh_balance(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> AppResult<u32> {
    tracing::info!("🔄 FETCH FRESH BALANCE - Direct HTTP call");

    // Build URL from configuration
    let url = {
        let config = state.config.lock().await;
        config.build_ledger_url()?
    };

    tracing::info!("🔄 Using dynamic URL: {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("HTTP error: {}", response.status())
        ).into());
    }

    let json: serde_json::Value = response.json().await?;
    tracing::info!("🔄 FETCH FRESH BALANCE - Response: {:?}", json);

    // Extract balance from JSON
    if let Some(balance_value) = json.get("credits_balance") {
        if let Some(balance_str) = balance_value.as_str() {
            if let Ok(balance) = balance_str.parse::<f64>() {
                let balance_credits = balance as u32;
                tracing::info!("✅ FETCH FRESH BALANCE - Extracted: {} credits", balance_credits);

                // Store in database for analytics
                tracing::info!("💾 Storing fresh balance in database...");
                if let Err(e) = state.database.insert_balance_record(balance_credits).await {
                    tracing::error!("❌ Failed to store balance in database: {}", e);
                }

                // Update system tray
                tracing::info!("🎯 Updating system tray with fresh balance...");
                if let Err(e) = update_system_tray_balance(&app_handle, balance_credits) {
                    tracing::error!("❌ Failed to update system tray: {}", e);
                }

                // Emit event to frontend
                tracing::info!("📡 Emitting balance-updated event to frontend");
                if let Err(e) = app_handle.emit("balance-updated", balance_credits) {
                    tracing::error!("❌ Failed to emit balance-updated event: {}", e);
                } else {
                    tracing::info!("✅ Event emitted successfully");
                }

                return Ok(balance_credits);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to extract balance from response"
    ).into())
}

#[tauri::command]
async fn get_usage_analytics(
    state: tauri::State<'_, AppState>,
    hours: Option<u32>,
) -> AppResult<analytics::UsageAnalytics> {
    let hours = hours.unwrap_or(24);
    let analytics = state.analytics.calculate_usage_analytics(hours).await?;
    Ok(analytics)
}

#[tauri::command]
async fn update_config(
    state: tauri::State<'_, AppState>,
    new_config: config::AppConfig,
) -> AppResult<()> {
    let mut config = state.config.lock().await;
    *config = new_config;
    config.save().await?;
    Ok(())
}

#[tauri::command]
async fn trigger_manual_update(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> AppResult<Option<u32>> {
    tracing::info!("🔄 MANUAL UPDATE TRIGGERED");

    // Use a shorter scope for the config lock to avoid deadlock
    let token = {
        let config = state.config.lock().await;
        tracing::info!("🔍 Config loaded, checking token...");
        tracing::info!("🔍 Token present: {}", config.orb_token.is_some());
        config.orb_token.clone()
    };

    if let Some(token) = token {
        tracing::info!("🔍 Token found: {}...", &token[..std::cmp::min(20, token.len())]);
        tracing::info!("🔄 Manual update: Fetching balance...");
        let balance = state.scraper.fetch_balance(&token).await?;
        tracing::info!("✅ Manual update: Successfully fetched balance: {}", balance);

        // Store in database
        tracing::info!("💾 Storing balance in database...");
        state.database.insert_balance_record(balance).await?;
        tracing::info!("✅ Balance stored in database");

        // Update system tray
        tracing::info!("🎯 Updating system tray...");
        if let Err(e) = update_system_tray_balance(&app_handle, balance) {
            tracing::error!("❌ Failed to update system tray during manual update: {}", e);
        } else {
            tracing::info!("✅ System tray updated during manual update");
        }

        // Emit event to frontend
        tracing::info!("📡 Emitting event to frontend...");
        if let Err(e) = app_handle.emit("balance-updated", balance) {
            tracing::error!("❌ Failed to emit balance update event during manual update: {}", e);
        } else {
            tracing::info!("✅ Event emitted to frontend during manual update");
        }

        tracing::info!("🎉 Manual update: COMPLETED successfully with balance: {}", balance);
        Ok(Some(balance))
    } else {
        tracing::warn!("⚠️ Manual update: No token configured");
        Ok(None)
    }
}

#[tauri::command]
async fn update_tray_balance(app_handle: tauri::AppHandle, balance: u32) -> AppResult<()> {
    // Format balance for display
    let balance_text = if balance > 9999 {
        format!("{}k", balance / 1000)
    } else {
        balance.to_string()
    };

    // Get the tray icon by ID
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        // Set the title to show the balance directly in the menu bar (macOS)
        tray.set_title(Some(&balance_text)).map_err(|e| error::AppError::Unknown(e.to_string()))?;

        // Also set tooltip for additional info
        tray.set_tooltip(Some(&format!("{} - Augment Credits (Click to toggle)", balance_text))).map_err(|e| error::AppError::Unknown(e.to_string()))?;
    }

    Ok(())
}

#[tauri::command]
async fn show_window(app_handle: tauri::AppHandle) -> AppResult<()> {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.show().map_err(|e| error::AppError::Unknown(e.to_string()))?;
        window.set_focus().map_err(|e| error::AppError::Unknown(e.to_string()))?;

        // Update window visibility state
        if let Some(state) = app_handle.try_state::<AppState>() {
            if let Ok(mut visible) = state.window_visible.try_lock() {
                *visible = true;
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn hide_window(app_handle: tauri::AppHandle) -> AppResult<()> {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.hide().map_err(|e| error::AppError::Unknown(e.to_string()))?;

        // Update window visibility state
        if let Some(state) = app_handle.try_state::<AppState>() {
            if let Ok(mut visible) = state.window_visible.try_lock() {
                *visible = false;
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn get_window_visibility(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    let visible = state.window_visible.lock().await;
    Ok(*visible)
}

#[tauri::command]
async fn toggle_window(app_handle: tauri::AppHandle) -> AppResult<bool> {
    if let Some(state) = app_handle.try_state::<AppState>() {
        if let Ok(mut visible) = state.window_visible.try_lock() {
            if *visible {
                if let Some(window) = app_handle.get_webview_window("main") {
                    window.hide().map_err(|e| error::AppError::Unknown(e.to_string()))?;
                    *visible = false;
                }
            } else {
                if let Some(window) = app_handle.get_webview_window("main") {
                    window.show().map_err(|e| error::AppError::Unknown(e.to_string()))?;
                    window.set_focus().map_err(|e| error::AppError::Unknown(e.to_string()))?;
                    *visible = true;
                }
            }
            return Ok(*visible);
        }
    }
    Ok(false)
}

// Test event emission function removed for production

#[tauri::command]
async fn parse_orb_url(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    url: String,
) -> AppResult<()> {
    tracing::info!("🔍 PARSE ORB URL: {}", url);

    let mut config = state.config.lock().await;
    config.parse_orb_url(&url)?;

    // Save the updated configuration
    config.save().await?;

    tracing::info!("✅ ORB URL parsed and configuration saved");

    // Immediately fetch fresh balance to show user instant feedback
    if let Some(token) = config.orb_token.clone() {
        tracing::info!("🔄 IMMEDIATE FETCH: Getting fresh balance after URL setup...");

        // Release the config lock before making the API call
        drop(config);

        match state.scraper.fetch_balance(&token).await {
            Ok(balance) => {
                tracing::info!("✅ IMMEDIATE FETCH: Successfully fetched balance: {}", balance);

                // Store in database
                if let Err(e) = state.database.insert_balance_record(balance).await {
                    tracing::error!("❌ Failed to store immediate balance in database: {}", e);
                }

                // Update system tray
                if let Err(e) = update_system_tray_balance(&app_handle, balance) {
                    tracing::error!("❌ Failed to update system tray during immediate fetch: {}", e);
                } else {
                    tracing::info!("✅ System tray updated during immediate fetch");
                }

                // Emit event to frontend
                if let Err(e) = app_handle.emit("balance-updated", balance) {
                    tracing::error!("❌ Failed to emit immediate balance update event: {}", e);
                } else {
                    tracing::info!("✅ Immediate balance update event emitted successfully");
                }
            }
            Err(e) => {
                tracing::error!("❌ IMMEDIATE FETCH: Failed to fetch balance after URL setup: {}", e);
                // Don't fail the entire operation, just log the error
                // The background monitoring will retry later
            }
        }
    } else {
        tracing::warn!("⚠️ No token found after URL parsing - this shouldn't happen");
    }

    Ok(())
}

#[tauri::command]
async fn get_orb_config(
    state: tauri::State<'_, AppState>,
) -> AppResult<serde_json::Value> {
    let config = state.config.lock().await;

    Ok(serde_json::json!({
        "customer_id": config.customer_id,
        "pricing_unit_id": config.pricing_unit_id,
        "has_token": config.orb_token.is_some(),
        "is_configured": config.is_orb_configured()
    }))
}

#[tauri::command]
async fn clear_orb_config(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> AppResult<()> {
    tracing::info!("🗑️ CLEAR ORB CONFIG");

    let mut config = state.config.lock().await;
    config.customer_id = None;
    config.pricing_unit_id = None;
    config.orb_token = None;

    // Save the updated configuration
    config.save().await?;

    // Clear the system tray to remove any displayed balance
    tracing::info!("🗑️ Clearing system tray display...");
    if let Err(e) = clear_system_tray(&app_handle) {
        tracing::error!("❌ Failed to clear system tray: {}", e);
        // Don't fail the entire operation if tray clearing fails
    } else {
        tracing::info!("✅ System tray cleared successfully");
    }

    tracing::info!("✅ ORB configuration cleared");
    Ok(())
}

#[tauri::command]
async fn clear_system_tray_command(app_handle: tauri::AppHandle) -> AppResult<()> {
    tracing::info!("🗑️ CLEAR SYSTEM TRAY COMMAND called from frontend");

    if let Err(e) = clear_system_tray(&app_handle) {
        tracing::error!("❌ Failed to clear system tray via command: {}", e);
        return Err(error::AppError::Unknown(format!("Failed to clear system tray: {}", e)));
    }

    tracing::info!("✅ System tray cleared successfully via command");
    Ok(())
}

// ============================================================================
// NEW AUGMENT API COMMANDS
// ============================================================================

/// Save the session cookie from WebView login
#[tauri::command]
async fn save_session_cookie(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    session_cookie: String,
) -> AppResult<serde_json::Value> {
    tracing::info!("🔐 SAVE SESSION COOKIE - Validating and saving...");

    // Create client and validate the session
    let client = AugmentClient::new(session_cookie.clone())?;

    // Fetch user info to validate and get email
    let user = client.fetch_user().await?;
    tracing::info!("✅ Session validated for user: {}", user.email);

    // Save to config
    {
        let mut config = state.config.lock().await;
        config.set_session_cookie(session_cookie, Some(user.email.clone()));
        config.save().await?;
    }

    // Immediately fetch balance
    let credits = client.fetch_credits().await?;
    let balance = credits.usage_units_remaining as u32;

    // Store in database
    if let Err(e) = state.database.insert_balance_record(balance).await {
        tracing::error!("❌ Failed to store balance: {}", e);
    }

    // Update system tray
    if let Err(e) = update_system_tray_balance(&app_handle, balance) {
        tracing::error!("❌ Failed to update tray: {}", e);
    }

    // Emit event to frontend
    let _ = app_handle.emit("balance-updated", balance);

    Ok(serde_json::json!({
        "success": true,
        "email": user.email,
        "balance": balance
    }))
}

/// Fetch credits using new Augment API
#[tauri::command]
async fn fetch_augment_credits(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> AppResult<serde_json::Value> {
    tracing::info!("🔄 FETCH AUGMENT CREDITS");

    let session_cookie = {
        let config = state.config.lock().await;
        config.session_cookie.clone()
    };

    let session_cookie = session_cookie.ok_or_else(|| {
        AppError::Auth("No session configured".to_string())
    })?;

    let client = AugmentClient::new(session_cookie)?;
    let credits = client.fetch_credits().await?;
    let balance = credits.usage_units_remaining as u32;

    // Store in database
    if let Err(e) = state.database.insert_balance_record(balance).await {
        tracing::error!("❌ Failed to store balance: {}", e);
    }

    // Update system tray
    if let Err(e) = update_system_tray_balance(&app_handle, balance) {
        tracing::error!("❌ Failed to update tray: {}", e);
    }

    // Emit event to frontend
    let _ = app_handle.emit("balance-updated", balance);

    Ok(serde_json::json!({
        "credits_remaining": credits.usage_units_remaining,
        "credits_used": credits.usage_units_consumed_this_billing_cycle,
        "balance": balance
    }))
}

/// Fetch subscription info using new Augment API
#[tauri::command]
async fn fetch_augment_subscription(
    state: tauri::State<'_, AppState>,
) -> AppResult<serde_json::Value> {
    tracing::info!("🔄 FETCH AUGMENT SUBSCRIPTION");

    let session_cookie = {
        let config = state.config.lock().await;
        config.session_cookie.clone()
    };

    let session_cookie = session_cookie.ok_or_else(|| {
        AppError::Auth("No session configured".to_string())
    })?;

    let client = AugmentClient::new(session_cookie)?;
    let subscription = client.fetch_subscription().await?;

    Ok(serde_json::json!({
        "plan_name": subscription.plan_name,
        "billing_period_end": subscription.billing_period_end,
        "credits_included": subscription.credits_included_this_billing_cycle,
        "credits_renewing": subscription.credits_renewing_each_billing_cycle,
        "trial_grant": subscription.trial_grant,
        "plan_facts": subscription.plan_facts
    }))
}

/// Fetch complete analytics data using new Augment API endpoints
#[tauri::command]
async fn fetch_augment_analytics(
    state: tauri::State<'_, AppState>,
    days: Option<u32>,
) -> AppResult<serde_json::Value> {
    let days = days.unwrap_or(30);
    tracing::info!("🔄 FETCH AUGMENT ANALYTICS (last {} days)", days);

    let session_cookie = {
        let config = state.config.lock().await;
        config.session_cookie.clone()
    };

    let session_cookie = session_cookie.ok_or_else(|| {
        AppError::Auth("No session cookie configured".to_string())
    })?;

    let client = AugmentClient::new(session_cookie)?;

    // Fetch all data in parallel
    let (analytics_info, daily_consumption, model_consumption, activity_consumption) = tokio::join!(
        client.fetch_credit_analytics_info(days),
        client.fetch_daily_consumption(days),
        client.fetch_consumption_by_model(days),
        client.fetch_consumption_by_activity(days)
    );

    // Process analytics info
    let analytics_info = analytics_info.unwrap_or_else(|e| {
        tracing::warn!("⚠️ Failed to fetch analytics info: {}", e);
        augment_client::CreditAnalyticsInfoResponse {
            total_credits_consumed: "0".to_string(),
            credits_percent_increase_over_previous_period: None,
            active_user_count: None,
            users_percent_increase_over_previous_period: None,
        }
    });

    // Process daily consumption
    let daily_usage = daily_consumption
        .map(|c| client.to_daily_usage(&c))
        .unwrap_or_else(|e| {
            tracing::warn!("⚠️ Failed to fetch daily consumption: {}", e);
            vec![]
        });

    // Process model consumption
    let model_usage = model_consumption
        .map(|c| client.to_model_usage(&c))
        .unwrap_or_else(|e| {
            tracing::warn!("⚠️ Failed to fetch model consumption: {}", e);
            vec![]
        });

    // Process activity consumption
    let activity_usage = activity_consumption
        .map(|c| client.to_activity_usage(&c))
        .unwrap_or_else(|e| {
            tracing::warn!("⚠️ Failed to fetch activity consumption: {}", e);
            vec![]
        });

    // Calculate summary stats from daily usage
    let total_credits_used: i64 = daily_usage.iter().map(|d| d.total_credits).sum();
    let days_with_data = daily_usage.len();
    let avg_daily_usage = if days_with_data > 0 {
        total_credits_used as f64 / days_with_data as f64
    } else {
        0.0
    };

    tracing::info!("✅ Analytics fetched: {} total, {} days, {} models, {} activities",
        analytics_info.total_credits_consumed,
        days_with_data,
        model_usage.len(),
        activity_usage.len()
    );

    Ok(serde_json::json!({
        "analytics_info": {
            "total_credits_consumed": analytics_info.total_credits_consumed.parse::<i64>().unwrap_or(0),
            "percent_increase": analytics_info.credits_percent_increase_over_previous_period,
            "active_users": analytics_info.active_user_count.unwrap_or(1)
        },
        "daily_usage": daily_usage,
        "model_usage": model_usage,
        "activity_usage": activity_usage,
        "summary": {
            "total_credits_used": total_credits_used,
            "days_with_data": days_with_data,
            "avg_daily_usage": avg_daily_usage,
            "period_days": days
        }
    }))
}

/// Get current auth status
#[tauri::command]
async fn get_auth_status(
    state: tauri::State<'_, AppState>,
) -> AppResult<serde_json::Value> {
    let config = state.config.lock().await;

    Ok(serde_json::json!({
        "is_authenticated": config.is_authenticated(),
        "is_augment_configured": config.is_augment_configured(),
        "is_orb_configured": config.is_orb_configured(),
        "user_email": config.user_email,
        "auth_method": if config.is_augment_configured() {
            "augment"
        } else if config.is_orb_configured() {
            "orb"
        } else {
            "none"
        }
    }))
}

/// Clear Augment session (logout)
#[tauri::command]
async fn clear_augment_session(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> AppResult<()> {
    tracing::info!("🗑️ CLEAR AUGMENT SESSION");

    {
        let mut config = state.config.lock().await;
        config.clear_augment_session();
        config.save().await?;
    }

    // Clear system tray
    let _ = clear_system_tray(&app_handle);

    tracing::info!("✅ Augment session cleared");
    Ok(())
}

/// Helper function to validate and save session from the login WebView
async fn validate_and_save_session(app_handle: &tauri::AppHandle, session_cookie: String) -> AppResult<()> {
    tracing::info!("🔐 Validating session cookie...");

    // Validate the session by fetching user info
    let client = AugmentClient::new(session_cookie.clone())?;
    let user = client.fetch_user().await?;
    tracing::info!("✅ Session validated for user: {}", user.email);

    // Get state from app handle
    let state = app_handle.state::<AppState>();

    // Save to config
    {
        let mut config = state.config.lock().await;
        config.set_session_cookie(session_cookie, Some(user.email.clone()));
        config.save().await?;
    }

    // Fetch initial balance
    let credits = client.fetch_credits().await?;
    let balance = credits.usage_units_remaining as u32;

    // Store in database
    if let Err(e) = state.database.insert_balance_record(balance).await {
        tracing::error!("❌ Failed to store balance: {}", e);
    }

    // Update system tray
    if let Err(e) = update_system_tray_balance(app_handle, balance) {
        tracing::error!("❌ Failed to update tray: {}", e);
    }

    // Emit events
    let _ = app_handle.emit("balance-updated", balance);
    let _ = app_handle.emit("config-changed", ());

    Ok(())
}

/// Open a WebView window for Augment login
/// This creates a new window that loads app.augmentcode.com
/// After login, JavaScript extracts the _session cookie and sends it back
#[tauri::command]
async fn open_augment_login(app_handle: tauri::AppHandle) -> AppResult<()> {
    tracing::info!("🔐 OPEN AUGMENT LOGIN WEBVIEW");

    // Check if login window already exists
    if app_handle.get_webview_window("augment-login").is_some() {
        tracing::info!("Login window already exists, focusing it");
        if let Some(window) = app_handle.get_webview_window("augment-login") {
            let _ = window.set_focus();
        }
        return Ok(());
    }

    let app_handle_clone = app_handle.clone();
    let app_handle_for_nav = app_handle.clone();

    // Create the login window
    let login_window = WebviewWindowBuilder::new(
        &app_handle,
        "augment-login",
        WebviewUrl::External("https://app.augmentcode.com".parse().unwrap()),
    )
    .title("Login to Augment")
    .inner_size(480.0, 700.0)
    .center()
    .resizable(true)
    .on_navigation(move |url| {
        let url_str = url.as_str();

        // Intercept our special path for cookie extraction (no custom protocol = no OS dialog)
        if url_str.contains("/__tauri_extract_session__") {
            tracing::info!("🔗 Intercepted session extraction request");
            let app_handle = app_handle_for_nav.clone();

            // Extract cookies from WebView's cookie store
            tracing::info!("🔒 Extracting session cookie from WebView cookie store...");

            if let Some(login_win) = app_handle.get_webview_window("augment-login") {
                let augment_url = url::Url::parse("https://app.augmentcode.com").unwrap();
                match login_win.cookies_for_url(augment_url) {
                    Ok(cookies) => {
                        tracing::info!("🍪 Found {} cookies", cookies.len());

                        // Find the _session cookie
                        if let Some(session_cookie) = cookies.iter().find(|c| c.name() == "_session") {
                            let session_str = session_cookie.value().to_string();
                            tracing::info!("✅ Found _session cookie (len: {})", session_str.len());

                            tauri::async_runtime::spawn(async move {
                                match validate_and_save_session(&app_handle, session_str).await {
                                    Ok(_) => {
                                        tracing::info!("✅ Session validated and saved!");
                                        let _ = app_handle.emit("login-success", ());
                                        if let Some(login_win) = app_handle.get_webview_window("augment-login") {
                                            let _ = login_win.close();
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("❌ Session validation failed: {}", e);
                                        let _ = app_handle.emit("login-error", e.to_string());
                                    }
                                }
                            });
                        } else {
                            tracing::error!("❌ _session cookie not found in cookie store");
                            let _ = app_handle.emit("login-error", "Session cookie not found. Please try logging in again.".to_string());
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to get cookies: {}", e);
                        let _ = app_handle.emit("login-error", format!("Failed to extract session: {}", e));
                    }
                }
            }

            // Block navigation - don't actually go to this URL
            return false;
        }

        // Allow all other navigations
        true
    })
    .on_page_load(move |webview, payload| {
        if let PageLoadEvent::Finished = payload.event() {
            let url = payload.url().to_string();
            tracing::info!("📄 Page loaded: {}", url);

            // If we're on app.augmentcode.com (not login page), try to extract cookie
            if url.starts_with("https://app.augmentcode.com") && !url.contains("login") && !url.contains("auth") {
                tracing::info!("🎉 User is on app.augmentcode.com - injecting cookie extraction UI...");

                let app_handle_for_js = app_handle_clone.clone();
                let webview_clone = webview.clone();

                // Inject a floating button that extracts and displays the cookie
                tauri::async_runtime::spawn(async move {
                    // Wait for page to fully load
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

                    // JavaScript to show a modal - the cookie will be sent via document.cookie or we'll use a workaround
                    let inject_ui_js = r#"
                        (function() {
                            // Check if we already injected
                            if (document.getElementById('__tauri_cookie_modal__')) return;

                            function getCookie(name) {
                                const value = `; ${document.cookie}`;
                                const parts = value.split(`; ${name}=`);
                                if (parts.length === 2) return parts.pop().split(';').shift();
                                return null;
                            }

                            // Try to get cookie - it might be HttpOnly so we'll use a workaround
                            let sessionCookie = getCookie('_session');

                            // If cookie is HttpOnly, we'll signal with a special marker
                            // and Tauri will need to extract cookies via another method
                            const isHttpOnly = !sessionCookie || sessionCookie.length < 100;

                            console.log('Session cookie accessible via JS:', !isHttpOnly);
                            console.log('Cookie length:', sessionCookie ? sessionCookie.length : 0);

                            // Create modal overlay
                            const overlay = document.createElement('div');
                            overlay.id = '__tauri_cookie_modal__';
                            overlay.style.cssText = `
                                position: fixed;
                                top: 0;
                                left: 0;
                                right: 0;
                                bottom: 0;
                                background: rgba(0,0,0,0.8);
                                display: flex;
                                align-items: center;
                                justify-content: center;
                                z-index: 999999;
                                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                            `;

                            // Create modal content
                            const modal = document.createElement('div');
                            modal.style.cssText = `
                                background: #1a1a2e;
                                border-radius: 16px;
                                padding: 32px;
                                max-width: 500px;
                                width: 90%;
                                text-align: center;
                                box-shadow: 0 20px 60px rgba(0,0,0,0.5);
                                border: 1px solid #333;
                            `;

                            modal.innerHTML = `
                                <div style="font-size: 48px; margin-bottom: 16px;">🎉</div>
                                <h2 style="color: #fff; margin: 0 0 8px 0; font-size: 24px;">Login Successful!</h2>
                                <p style="color: #888; margin: 0 0 24px 0; font-size: 14px;">
                                    Click the button below to connect your account to the app.
                                </p>
                                <button id="__tauri_connect_btn__" style="
                                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                                    color: white;
                                    border: none;
                                    padding: 16px 32px;
                                    font-size: 16px;
                                    font-weight: 600;
                                    border-radius: 12px;
                                    cursor: pointer;
                                    width: 100%;
                                    transition: transform 0.2s, box-shadow 0.2s;
                                ">
                                    🔗 Connect to App
                                </button>
                                <p id="__tauri_status__" style="color: #4ade80; margin: 16px 0 0 0; font-size: 14px; display: none;">
                                    ✅ Connected! This window will close automatically.
                                </p>
                            `;

                            overlay.appendChild(modal);
                            document.body.appendChild(overlay);

                            // Add click handler - navigate to hash URL that Tauri intercepts
                            document.getElementById('__tauri_connect_btn__').addEventListener('click', function() {
                                this.textContent = '⏳ Connecting...';
                                this.disabled = true;

                                // Also show status
                                const status = document.getElementById('__tauri_status__');
                                if (status) {
                                    status.style.display = 'block';
                                    status.textContent = '⏳ Extracting session...';
                                }

                                // Navigate to a page on the same domain with special path
                                // This won't trigger external app dialog
                                window.location.href = 'https://app.augmentcode.com/__tauri_extract_session__';
                            });

                            // Hover effect
                            const btn = document.getElementById('__tauri_connect_btn__');
                            btn.addEventListener('mouseenter', () => {
                                btn.style.transform = 'scale(1.02)';
                                btn.style.boxShadow = '0 8px 30px rgba(102, 126, 234, 0.4)';
                            });
                            btn.addEventListener('mouseleave', () => {
                                btn.style.transform = 'scale(1)';
                                btn.style.boxShadow = 'none';
                            });
                        })();
                    "#;

                    if let Err(e) = webview_clone.eval(inject_ui_js) {
                        tracing::error!("❌ Failed to inject cookie UI: {}", e);
                    } else {
                        tracing::info!("✅ Cookie extraction UI injected");
                    }
                });
            }
        }
    })
    .build()
    .map_err(|e| AppError::Unknown(format!("Failed to create login window: {}", e)))?;

    login_window.set_focus().map_err(|e| AppError::Unknown(e.to_string()))?;

    tracing::info!("✅ Login window opened");
    Ok(())
}

/// Receive cookie from the login WebView
#[tauri::command]
async fn receive_login_cookie(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    session_cookie: String,
) -> AppResult<serde_json::Value> {
    tracing::info!("🍪 RECEIVED LOGIN COOKIE from WebView");

    // Validate the session by fetching user info
    let client = AugmentClient::new(session_cookie.clone())?;
    let user = client.fetch_user().await?;
    tracing::info!("✅ Session validated for user: {}", user.email);

    // Save to config
    {
        let mut config = state.config.lock().await;
        config.set_session_cookie(session_cookie, Some(user.email.clone()));
        config.save().await?;
    }

    // Fetch initial balance
    let credits = client.fetch_credits().await?;
    let balance = credits.usage_units_remaining as u32;

    // Store in database
    if let Err(e) = state.database.insert_balance_record(balance).await {
        tracing::error!("❌ Failed to store balance: {}", e);
    }

    // Update system tray
    if let Err(e) = update_system_tray_balance(&app_handle, balance) {
        tracing::error!("❌ Failed to update tray: {}", e);
    }

    // Emit events
    let _ = app_handle.emit("balance-updated", balance);
    let _ = app_handle.emit("login-complete", serde_json::json!({
        "email": user.email,
        "balance": balance
    }));

    // Close the login window
    if let Some(login_window) = app_handle.get_webview_window("augment-login") {
        tracing::info!("🚪 Closing login window");
        let _ = login_window.close();
    }

    // Show main window
    if let Some(main_window) = app_handle.get_webview_window("main") {
        let _ = main_window.show();
        let _ = main_window.set_focus();
    }

    tracing::info!("✅ Login complete!");

    Ok(serde_json::json!({
        "success": true,
        "email": user.email,
        "balance": balance
    }))
}

fn create_system_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let balance = MenuItem::with_id(app, "balance", "Balance: Loading...", false, None::<&str>)?;
    let separator1 = MenuItem::with_id(app, "separator1", "---", false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide Window", true, None::<&str>)?;
    let separator2 = MenuItem::with_id(app, "separator2", "---", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Application", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&balance, &separator1, &show, &hide, &separator2, &quit])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("Augment Credits - Not logged in")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "quit" => {
                    app.exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();

                        // Update window visibility state
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut visible) = state.window_visible.try_lock() {
                                *visible = true;
                            }
                        }
                    }
                }
                "hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();

                        // Update window visibility state
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut visible) = state.window_visible.try_lock() {
                                *visible = false;
                            }
                        }
                    }
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    // Toggle window visibility
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(mut visible) = state.window_visible.try_lock() {
                            if *visible {
                                let _ = window.hide();
                                *visible = false;
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                                *visible = true;
                            }
                        }
                    } else {
                        // Fallback: just show the window
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

async fn setup_app_state() -> AppResult<AppState> {
    // Initialize configuration
    let config = Arc::new(Mutex::new(AppConfig::load().await?));
    
    // Initialize database
    let database = Arc::new(Database::new().await?);
    
    // Initialize scraper
    let scraper = Arc::new(orbScraper::new().await?);
    
    // Initialize analytics engine
    let analytics = Arc::new(AnalyticsEngine::new(database.clone()));
    
    // Initialize notification manager
    let notifications = Arc::new(Mutex::new(NotificationManager::new()));
    
    Ok(AppState {
        config,
        database,
        scraper,
        analytics,
        notifications,
        window_visible: Arc::new(Mutex::new(true)), // Start with window visible
    })
}

#[tokio::main]
async fn main() -> AppResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Setup application state
    let app_state = setup_app_state().await?;
    
    // We'll start the monitoring task after the app is built

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            test_connection,
            get_current_balance,
            get_usage_analytics,
            update_config,
            trigger_manual_update,
            update_tray_balance,
            fetch_fresh_balance,
            show_window,
            hide_window,
            get_window_visibility,
            toggle_window,
            parse_orb_url,
            get_orb_config,
            clear_orb_config,
            clear_system_tray_command,
            // New Augment API commands
            save_session_cookie,
            fetch_augment_credits,
            fetch_augment_subscription,
            fetch_augment_analytics,
            get_auth_status,
            clear_augment_session,
            open_augment_login,
            receive_login_cookie
        ])
        .setup(|app| {
            // Create system tray
            create_system_tray(&app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                window.show().unwrap(); // Show window initially

                // Set up window event handler to prevent app termination on close
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    match event {
                        WindowEvent::CloseRequested { api, .. } => {
                            // Prevent the window from closing and hide it instead
                            api.prevent_close();

                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.hide();

                                // Update window visibility state
                                if let Some(state) = app_handle.try_state::<AppState>() {
                                    if let Ok(mut visible) = state.window_visible.try_lock() {
                                        *visible = false;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                });
            }

            // Start background monitoring task with app handle
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();
            let state_clone = state.inner().clone();
            tokio::spawn(async move {
                monitoring_loop(state_clone, app_handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

async fn monitoring_loop(state: AppState, app_handle: tauri::AppHandle) {
    // Get polling interval from config, default to 60 seconds
    let polling_interval = {
        let config = state.config.lock().await;
        config.polling_interval_seconds
    };

    tracing::info!("🚀 MONITORING LOOP STARTED with {}s interval", polling_interval);
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(polling_interval as u64));

    loop {
        interval.tick().await;
        tracing::info!("⏰ MONITORING LOOP TICK - Starting new cycle");

        // Check auth method and get credentials
        let (session_cookie, orb_token) = {
            let config = state.config.lock().await;
            (config.session_cookie.clone(), config.orb_token.clone())
        };

        // Priority: Use new Augment API if session cookie is available
        if let Some(session_cookie) = session_cookie {
            tracing::info!("🔄 Background monitoring: Using Augment API...");
            match AugmentClient::new(session_cookie) {
                Ok(client) => {
                    match client.fetch_credits().await {
                        Ok(credits) => {
                            let balance = credits.usage_units_remaining as u32;
                            tracing::info!("✅ Background monitoring: Augment credits: {}", balance);

                            if let Err(e) = state.database.insert_balance_record(balance).await {
                                tracing::error!("❌ Failed to insert balance record: {}", e);
                            }

                            if let Err(e) = update_system_tray_balance(&app_handle, balance) {
                                tracing::error!("❌ Failed to update system tray: {}", e);
                            }

                            if let Err(e) = app_handle.emit("balance-updated", balance) {
                                tracing::error!("❌ Failed to emit balance event: {}", e);
                            }

                            // Check for alerts
                            if let Ok(analytics) = state.analytics.calculate_usage_analytics(24).await {
                                let mut notifications = state.notifications.lock().await;
                                notifications.check_and_send_alerts(&analytics, balance).await;
                            }
                        }
                        Err(e) => {
                            tracing::error!("❌ Augment API error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to create Augment client: {}", e);
                }
            }
        }
        // Fallback: Use legacy Orb scraper
        else if let Some(token) = orb_token {
            tracing::info!("🔄 Background monitoring: Using legacy Orb scraper...");
            match state.scraper.fetch_balance(&token).await {
                Ok(balance) => {
                    tracing::info!("✅ Background monitoring (Orb): balance: {}", balance);

                    if let Err(e) = state.database.insert_balance_record(balance).await {
                        tracing::error!("❌ Failed to insert balance record: {}", e);
                    }

                    if let Err(e) = update_system_tray_balance(&app_handle, balance) {
                        tracing::error!("❌ Failed to update system tray: {}", e);
                    }

                    if let Err(e) = app_handle.emit("balance-updated", balance) {
                        tracing::error!("❌ Failed to emit balance event: {}", e);
                    }

                    if let Ok(analytics) = state.analytics.calculate_usage_analytics(24).await {
                        let mut notifications = state.notifications.lock().await;
                        notifications.check_and_send_alerts(&analytics, balance).await;
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Orb scraper error: {}", e);
                }
            }
        } else {
            tracing::warn!("⚠️ Background monitoring: No auth configured, skipping fetch");

            if let Err(e) = clear_system_tray(&app_handle) {
                tracing::error!("❌ Failed to clear system tray: {}", e);
            }
        }

        tracing::info!("🔄 MONITORING LOOP CYCLE COMPLETE - Waiting for next tick");
    }
}

fn update_system_tray_balance(app_handle: &tauri::AppHandle, balance: u32) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("🎯 update_system_tray_balance called with balance: {}", balance);

    // Format balance for display
    let balance_text = if balance > 9999 {
        format!("{}k", balance / 1000)
    } else {
        balance.to_string()
    };

    tracing::info!("📝 Formatted balance text: '{}'", balance_text);

    // Get the tray icon by ID and update it
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        tracing::info!("✅ Found tray icon with ID 'main-tray'");

        // Set the title to show the balance directly in the menu bar (macOS)
        tracing::info!("🔄 Setting tray title to: '{}'", balance_text);
        tray.set_title(Some(&balance_text))?;
        tracing::info!("✅ Tray title set successfully");

        // Also set tooltip for additional info
        let tooltip = format!("{} - Augment Credits", balance_text);
        tracing::info!("🔄 Setting tray tooltip to: '{}'", tooltip);
        tray.set_tooltip(Some(&tooltip))?;
        tracing::info!("✅ Tray tooltip set successfully");
    } else {
        tracing::error!("❌ Could not find tray icon with ID 'main-tray'");
        return Err("Tray icon not found".into());
    }

    tracing::info!("✅ update_system_tray_balance completed successfully");
    Ok(())
}

fn clear_system_tray(app_handle: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("🗑️ clear_system_tray called - clearing tray display");

    // Get the tray icon by ID and clear it
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        tracing::info!("✅ Found tray icon with ID 'main-tray'");

        // Clear the title (no balance shown)
        tracing::info!("🔄 Clearing tray title");
        tray.set_title(Some(""))?;
        tracing::info!("✅ Tray title cleared successfully");

        // Set tooltip to indicate not logged in
        let tooltip = "Augment Credits - Not logged in";
        tracing::info!("🔄 Setting tray tooltip to: '{}'", tooltip);
        tray.set_tooltip(Some(tooltip))?;
        tracing::info!("✅ Tray tooltip set successfully");
    } else {
        tracing::error!("❌ Could not find tray icon with ID 'main-tray'");
        return Err("Tray icon not found".into());
    }

    tracing::info!("✅ clear_system_tray completed successfully");
    Ok(())
}














