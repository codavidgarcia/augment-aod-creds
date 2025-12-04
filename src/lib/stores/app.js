import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Application state stores
export const currentBalance = writable(null);
export const usageAnalytics = writable(null);
export const appConfig = writable({
  orb_token: null,
  customer_id: null,
  pricing_unit_id: null,
  polling_interval_seconds: 60,
  low_balance_threshold: 500,
  critical_balance_threshold: 100,
  enable_notifications: true,
  enable_sound_alerts: false,
  auto_start: false,
  window_always_on_top: false,
  compact_mode: true,
  theme: 'System',
  data_retention_days: 30,
});

// Orb configuration state (legacy)
export const orbConfig = writable({
  customer_id: null,
  pricing_unit_id: null,
  has_token: false,
  is_configured: false
});

// Auth status (new)
export const authStatus = writable({
  is_authenticated: false,
  is_augment_configured: false,
  is_orb_configured: false,
  user_email: null,
  auth_method: 'none'
});

// Subscription info
export const subscriptionInfo = writable(null);

// Credit consumption history (from Augment API)
export const consumptionHistory = writable(null);

export const isLoading = writable(false);
export const lastUpdate = writable(null);
export const connectionStatus = writable('disconnected'); // 'connected', 'disconnected', 'error'
export const alerts = writable([]);

// Derived stores
export const balanceStatus = derived(
  [currentBalance, appConfig],
  ([$currentBalance, $appConfig]) => {
    if (!$currentBalance) return 'unknown';
    
    if ($currentBalance <= $appConfig.critical_balance_threshold) {
      return 'critical';
    } else if ($currentBalance <= $appConfig.low_balance_threshold) {
      return 'warning';
    } else {
      return 'healthy';
    }
  }
);

export const balanceStatusColor = derived(balanceStatus, ($balanceStatus) => {
  switch ($balanceStatus) {
    case 'critical':
      return 'text-danger-600 dark:text-danger-400';
    case 'warning':
      return 'text-warning-600 dark:text-warning-400';
    case 'healthy':
      return 'text-success-600 dark:text-success-400';
    default:
      return 'text-gray-600 dark:text-gray-400';
  }
});

export const formattedBalance = derived(currentBalance, ($currentBalance) => {
  if ($currentBalance === null || $currentBalance === undefined) {
    return '--';
  }
  return $currentBalance.toLocaleString();
});

export const usageRate = derived(usageAnalytics, ($usageAnalytics) => {
  if (!$usageAnalytics) return null;
  return {
    perHour: $usageAnalytics.usage_rate_per_hour,
    perDay: $usageAnalytics.usage_rate_per_day,
  };
});

export const timeRemaining = derived(usageAnalytics, ($usageAnalytics) => {
  if (!$usageAnalytics) return null;
  return {
    hours: $usageAnalytics.estimated_hours_remaining,
    days: $usageAnalytics.estimated_days_remaining,
  };
});

// Actions
export const actions = {
  async fetchCurrentBalance() {
    console.log('📊 STORE: fetchCurrentBalance called - USING DIRECT HTTP CALL');
    try {
      console.log('📊 STORE: Setting isLoading to true');
      isLoading.set(true);

      console.log('📊 STORE: Calling invoke("fetch_fresh_balance") for GUARANTEED FRESH data...');
      const balance = await invoke('fetch_fresh_balance');
      console.log('📊 STORE: Received GUARANTEED FRESH balance from direct HTTP:', balance);

      console.log('📊 STORE: Setting currentBalance to:', balance);
      currentBalance.set(balance);

      console.log('📊 STORE: Setting lastUpdate to new Date');
      lastUpdate.set(new Date());

      console.log('📊 STORE: Setting connectionStatus to connected');
      connectionStatus.set('connected');

      console.log('📊 STORE: fetchCurrentBalance completed successfully with GUARANTEED FRESH data, returning:', balance);
      return balance;
    } catch (error) {
      console.error('❌ STORE: Failed to fetch GUARANTEED FRESH balance:', error);
      connectionStatus.set('error');
      throw error;
    } finally {
      console.log('📊 STORE: Setting isLoading to false');
      isLoading.set(false);
    }
  },

  async fetchInitialBalance() {
    console.log('📊 STORE: fetchInitialBalance called - USING DATABASE FOR FAST INIT');
    try {
      console.log('📊 STORE: Setting isLoading to true');
      isLoading.set(true);

      console.log('📊 STORE: Calling invoke("get_current_balance") for FAST init...');
      const balance = await invoke('get_current_balance');
      console.log('📊 STORE: Received balance from database:', balance);

      console.log('📊 STORE: Setting currentBalance to:', balance);
      currentBalance.set(balance);

      console.log('📊 STORE: Setting lastUpdate to new Date');
      lastUpdate.set(new Date());

      console.log('📊 STORE: Setting connectionStatus to connected');
      connectionStatus.set('connected');

      console.log('📊 STORE: fetchInitialBalance completed successfully, returning:', balance);
      return balance;
    } catch (error) {
      console.error('❌ STORE: Failed to fetch initial balance:', error);
      connectionStatus.set('error');
      throw error;
    } finally {
      console.log('📊 STORE: Setting isLoading to false');
      isLoading.set(false);
    }
  },

  async fetchUsageAnalytics(hours = 24) {
    try {
      const analytics = await invoke('get_usage_analytics', { hours });
      usageAnalytics.set(analytics);
      return analytics;
    } catch (error) {
      console.error('Failed to fetch analytics:', error);
      throw error;
    }
  },

  async updateConfig(newConfig) {
    try {
      console.log('Attempting to update config:', newConfig);
      await invoke('update_config', { newConfig });
      appConfig.set(newConfig);
      console.log('Config updated successfully');
      return true;
    } catch (error) {
      console.error('Failed to update config - detailed error:', error);
      console.error('Error type:', typeof error);
      console.error('Error message:', error.message || error);

      // Provide more specific error message
      let errorMessage = 'Unknown error occurred';
      if (error.message) {
        errorMessage = error.message;
      } else if (typeof error === 'string') {
        errorMessage = error;
      } else if (error.toString) {
        errorMessage = error.toString();
      }

      throw new Error(`Configuration update failed: ${errorMessage}`);
    }
  },

  async triggerManualUpdate() {
    try {
      isLoading.set(true);
      console.log('🔄 Triggering manual update...');
      const balance = await invoke('trigger_manual_update');
      console.log('📊 Manual update result:', balance);

      if (balance !== null && balance !== undefined) {
        currentBalance.set(balance);
        lastUpdate.set(new Date());
        connectionStatus.set('connected');
        console.log('✅ Manual update successful, balance:', balance);

        // Also refresh analytics
        await actions.fetchUsageAnalytics();
      } else {
        console.warn('⚠️ Manual update returned null/undefined - check token configuration');
        connectionStatus.set('error');
        throw new Error('No balance data received - please check your Orb token configuration');
      }
      return balance;
    } catch (error) {
      console.error('❌ Failed to trigger manual update:', error);
      connectionStatus.set('error');
      throw error;
    } finally {
      isLoading.set(false);
    }
  },

  formatTimeRemaining(hours) {
    if (!hours || hours <= 0) return 'Unknown';
    
    if (hours < 1) {
      const minutes = Math.round(hours * 60);
      return `${minutes}m`;
    } else if (hours < 24) {
      return `${hours.toFixed(1)}h`;
    } else {
      const days = Math.round(hours / 24);
      return `${days}d`;
    }
  },

  formatUsageRate(rate) {
    if (!rate || rate <= 0) return '0';

    if (rate < 1) {
      return rate.toFixed(2);
    } else if (rate < 10) {
      return rate.toFixed(1);
    } else {
      return Math.round(rate).toString();
    }
  },

  // Window management functions
  async showWindow() {
    try {
      await invoke('show_window');
      return true;
    } catch (error) {
      console.error('Failed to show window:', error);
      throw error;
    }
  },

  async hideWindow() {
    try {
      await invoke('hide_window');
      return true;
    } catch (error) {
      console.error('Failed to hide window:', error);
      throw error;
    }
  },

  async toggleWindow() {
    try {
      const isVisible = await invoke('toggle_window');
      return isVisible;
    } catch (error) {
      console.error('Failed to toggle window:', error);
      throw error;
    }
  },

  async parseOrbUrl(url) {
    try {
      console.log('🔍 Parsing Orb URL:', url);
      await invoke('parse_orb_url', { url });

      // Refresh the Orb configuration
      await actions.fetchOrbConfig();

      console.log('✅ Orb URL parsed successfully');
      return true;
    } catch (error) {
      console.error('❌ Failed to parse Orb URL:', error);
      throw error;
    }
  },

  async fetchOrbConfig() {
    try {
      console.log('📋 Calling get_orb_config...');

      // Add timeout to prevent hanging
      const configPromise = invoke('get_orb_config');
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('get_orb_config timeout after 10 seconds')), 10000)
      );

      const config = await Promise.race([configPromise, timeoutPromise]);
      orbConfig.set(config);
      console.log('📊 Orb config fetched successfully:', config);
      return config;
    } catch (error) {
      console.error('❌ Failed to fetch Orb config:', error);
      console.error('❌ Error details:', error.stack);

      // Set a default config to prevent complete failure
      const defaultConfig = {
        customer_id: null,
        pricing_unit_id: null,
        has_token: false,
        is_configured: false
      };
      orbConfig.set(defaultConfig);
      console.log('⚠️ Using default config due to error:', defaultConfig);
      return defaultConfig;
    }
  },

  async clearOrbConfig() {
    try {
      console.log('🗑️ Clearing Orb configuration...');
      await invoke('clear_orb_config');

      // Also explicitly clear the system tray from frontend
      console.log('🗑️ Clearing system tray from frontend...');
      try {
        await invoke('clear_system_tray_command');
        console.log('✅ System tray cleared from frontend');
      } catch (trayError) {
        console.warn('⚠️ Failed to clear system tray from frontend:', trayError);
        // Don't fail the entire logout process if tray clearing fails
      }

      // Reset the store
      orbConfig.set({
        customer_id: null,
        pricing_unit_id: null,
        has_token: false,
        is_configured: false
      });

      // Reset balance data
      currentBalance.set(null);
      connectionStatus.set('disconnected');

      // Reset usage analytics
      usageAnalytics.set(null);

      // Reset last update time
      lastUpdate.set(null);

      console.log('✅ Orb configuration cleared');
      return true;
    } catch (error) {
      console.error('❌ Failed to clear Orb config:', error);
      throw error;
    }
  },

  // ============================================================================
  // NEW AUGMENT API ACTIONS
  // ============================================================================

  /**
   * Save session cookie from WebView login
   */
  async saveSessionCookie(sessionCookie) {
    try {
      console.log('🔐 Saving session cookie...');
      isLoading.set(true);

      const result = await invoke('save_session_cookie', { sessionCookie });
      console.log('✅ Session saved:', result);

      // Update auth status
      await actions.fetchAuthStatus();

      // Update balance
      currentBalance.set(result.balance);
      lastUpdate.set(new Date());
      connectionStatus.set('connected');

      return result;
    } catch (error) {
      console.error('❌ Failed to save session cookie:', error);
      connectionStatus.set('error');
      throw error;
    } finally {
      isLoading.set(false);
    }
  },

  /**
   * Fetch auth status from backend
   */
  async fetchAuthStatus() {
    try {
      console.log('🔐 Fetching auth status...');
      const status = await invoke('get_auth_status');
      authStatus.set(status);
      console.log('✅ Auth status:', status);
      return status;
    } catch (error) {
      console.error('❌ Failed to fetch auth status:', error);
      throw error;
    }
  },

  /**
   * Fetch credits using new Augment API
   */
  async fetchAugmentCredits() {
    try {
      console.log('🔄 Fetching Augment credits...');
      isLoading.set(true);

      const result = await invoke('fetch_augment_credits');
      console.log('✅ Credits fetched:', result);

      currentBalance.set(result.balance);
      lastUpdate.set(new Date());
      connectionStatus.set('connected');

      return result;
    } catch (error) {
      console.error('❌ Failed to fetch Augment credits:', error);
      connectionStatus.set('error');
      throw error;
    } finally {
      isLoading.set(false);
    }
  },

  /**
   * Fetch subscription info
   */
  async fetchSubscription() {
    try {
      console.log('📋 Fetching subscription info...');
      const subscription = await invoke('fetch_augment_subscription');
      subscriptionInfo.set(subscription);
      console.log('✅ Subscription:', subscription);
      return subscription;
    } catch (error) {
      console.error('❌ Failed to fetch subscription:', error);
      throw error;
    }
  },

  /**
   * Fetch complete analytics from Augment API (daily, model, activity usage)
   */
  async fetchAugmentAnalytics(days = 30) {
    try {
      console.log(`📊 Fetching Augment analytics (last ${days} days)...`);

      const analytics = await invoke('fetch_augment_analytics', { days });

      console.log('✅ Augment analytics received:', analytics);

      // Store the raw consumption data
      consumptionHistory.set(analytics);

      // Get current balance for projections
      let currentBal = null;
      currentBalance.subscribe(val => currentBal = val)();

      // Extract data from new API response
      const dailyData = analytics.daily_usage || [];
      const modelData = analytics.model_usage || [];
      const activityData = analytics.activity_usage || [];
      const summary = analytics.summary || {};
      const analyticsInfo = analytics.analytics_info || {};

      // Calculate analytics from Augment data
      const avgDailyUsage = summary.avg_daily_usage || 0;
      const avgHourlyUsage = avgDailyUsage / 24;

      // Calculate estimated time remaining
      let estimatedDaysRemaining = null;
      let estimatedHoursRemaining = null;
      if (currentBal && avgDailyUsage > 0) {
        estimatedDaysRemaining = currentBal / avgDailyUsage;
        estimatedHoursRemaining = estimatedDaysRemaining * 24;
      }

      // Determine trend from percent increase
      let trend = 'Stable';
      if (analyticsInfo.percent_increase !== null && analyticsInfo.percent_increase !== undefined) {
        if (analyticsInfo.percent_increase > 10) trend = 'Increasing';
        else if (analyticsInfo.percent_increase < -10) trend = 'Decreasing';
      } else if (dailyData.length >= 3) {
        const recent = dailyData.slice(-3);
        const older = dailyData.slice(-6, -3);
        if (older.length > 0) {
          const recentAvg = recent.reduce((s, d) => s + d.total_credits, 0) / recent.length;
          const olderAvg = older.reduce((s, d) => s + d.total_credits, 0) / older.length;
          if (recentAvg > olderAvg * 1.1) trend = 'Increasing';
          else if (recentAvg < olderAvg * 0.9) trend = 'Decreasing';
        }
      } else if (dailyData.length < 3) {
        trend = 'Insufficient';
      }

      // Build analytics object compatible with existing UI
      const usageAnalyticsData = {
        current_balance: currentBal,
        usage_rate_per_hour: avgHourlyUsage,
        usage_rate_per_day: avgDailyUsage,
        estimated_days_remaining: estimatedDaysRemaining,
        estimated_hours_remaining: estimatedHoursRemaining,
        total_usage_period: (summary.days_with_data || 0) * 24,
        average_session_usage: avgDailyUsage,
        peak_usage_hour: null,
        trend: trend,
        efficiency_score: 75,
        // New: usage breakdown by model and activity
        model_usage: modelData,
        activity_usage: activityData,
        total_credits_consumed: analyticsInfo.total_credits_consumed || 0,
        percent_increase: analyticsInfo.percent_increase,
        // Balance and usage history for charts
        balance_history: dailyData.map(d => ({
          timestamp: d.date,
          balance: d.total_credits
        })),
        usage_history: dailyData.map(d => ({
          timestamp: d.date,
          usage_amount: d.total_credits,
          rate_per_hour: d.total_credits / 24
        }))
      };

      usageAnalytics.set(usageAnalyticsData);
      console.log('✅ Updated usageAnalytics from Augment API');

      return analytics;
    } catch (error) {
      console.warn('⚠️ Failed to fetch Augment analytics:', error);
      actions.setDefaultAnalytics();
      return null;
    }
  },

  /**
   * Set default analytics when consumption data is not available
   */
  setDefaultAnalytics() {
    let currentBal = null;
    currentBalance.subscribe(val => currentBal = val)();

    const analytics = {
      current_balance: currentBal,
      usage_rate_per_hour: 0,
      usage_rate_per_day: 0,
      estimated_days_remaining: null,
      estimated_hours_remaining: null,
      total_usage_period: 0,
      average_session_usage: 0,
      peak_usage_hour: null,
      trend: 'Insufficient',
      efficiency_score: 0,
      balance_history: [],
      usage_history: []
    };

    usageAnalytics.set(analytics);
    console.log('📊 Set default analytics (no consumption data available)');
  },

  /**
   * Clear Augment session (logout)
   */
  async clearAugmentSession() {
    try {
      console.log('🗑️ Clearing Augment session...');
      await invoke('clear_augment_session');

      // Reset stores
      authStatus.set({
        is_authenticated: false,
        is_augment_configured: false,
        is_orb_configured: false,
        user_email: null,
        auth_method: 'none'
      });
      currentBalance.set(null);
      connectionStatus.set('disconnected');
      usageAnalytics.set(null);
      subscriptionInfo.set(null);
      lastUpdate.set(null);

      console.log('✅ Augment session cleared');
      return true;
    } catch (error) {
      console.error('❌ Failed to clear Augment session:', error);
      throw error;
    }
  },

  async getWindowVisibility() {
    try {
      const isVisible = await invoke('get_window_visibility');
      return isVisible;
    } catch (error) {
      console.error('Failed to get window visibility:', error);
      throw error;
    }
  },

  getBalanceIcon(status) {
    switch (status) {
      case 'critical':
        return '🔴';
      case 'warning':
        return '🟡';
      case 'healthy':
        return '🟢';
      default:
        return '⚪';
    }
  },

  getTrendIcon(trend) {
    switch (trend) {
      case 'Increasing':
        return '📈';
      case 'Decreasing':
        return '📉';
      case 'Stable':
        return '➡️';
      default:
        return '❓';
    }
  }
};

// Auto-refresh functionality
let refreshInterval = null;

export function startAutoRefresh(intervalSeconds = 60) {
  stopAutoRefresh();

  console.log(`Starting auto-refresh with ${intervalSeconds}s interval`);

  refreshInterval = setInterval(async () => {
    try {
      console.log('⏰ AUTO-REFRESH: Starting auto-refresh cycle...');

      // Check which API to use
      let authStatusData = null;
      authStatus.subscribe(val => authStatusData = val)();

      if (authStatusData?.is_augment_configured) {
        // Use Augment API
        console.log('⏰ AUTO-REFRESH: Using Augment API...');
        await actions.fetchAugmentCredits();
        console.log('⏰ AUTO-REFRESH: Augment credits fetched');

        // Also refresh analytics
        try {
          await actions.fetchAugmentAnalytics(30);
          console.log('⏰ AUTO-REFRESH: Augment analytics fetched');
        } catch (e) {
          console.warn('⚠️ AUTO-REFRESH: Analytics fetch failed:', e);
        }
      } else if (authStatusData?.is_orb_configured) {
        // Use legacy Orb API
        console.log('⏰ AUTO-REFRESH: Using Orb API...');
        await actions.fetchCurrentBalance();
        console.log('⏰ AUTO-REFRESH: Orb balance fetched');

        await actions.fetchUsageAnalytics();
        console.log('⏰ AUTO-REFRESH: Orb analytics fetched');
      } else {
        console.log('⏰ AUTO-REFRESH: No auth configured, skipping');
        return;
      }

      console.log('⏰ AUTO-REFRESH: Cycle completed successfully');
    } catch (error) {
      console.error('❌ AUTO-REFRESH: Auto-refresh failed:', error);
      connectionStatus.set('error');
    }
  }, intervalSeconds * 1000);

  console.log('Auto-refresh started successfully');
}

export function stopAutoRefresh() {
  if (refreshInterval) {
    clearInterval(refreshInterval);
    refreshInterval = null;
  }
}

// Initialize app
export async function initializeApp(useFreshData = false) {
  try {
    console.log('🚀 Starting app initialization...', useFreshData ? '(with fresh data)' : '(with cached data)');

    // Test basic Tauri connectivity first
    console.log('🔌 Testing Tauri connectivity...');
    try {
      const connectPromise = invoke('test_connection');
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Connection test timeout')), 5000)
      );
      await Promise.race([connectPromise, timeoutPromise]);
      console.log('✅ Tauri connection test passed');
    } catch (error) {
      console.error('❌ Tauri connection test failed:', error);
      throw new Error('Failed to connect to Tauri backend: ' + error.message);
    }

    // Fetch auth status to determine which API to use
    console.log('🔐 Fetching auth status...');
    const authStatusData = await actions.fetchAuthStatus();
    console.log('✅ Auth status:', authStatusData);

    // Also fetch legacy Orb config for backward compatibility
    console.log('📋 Fetching Orb configuration...');
    const orbConfigData = await actions.fetchOrbConfig();
    console.log('✅ Orb config fetched:', orbConfigData);

    const isAuthenticated = authStatusData.is_authenticated || orbConfigData.is_configured;

    if (isAuthenticated) {
      console.log('✅ User is authenticated, proceeding with full initialization');

      try {
        if (authStatusData.is_augment_configured) {
          // Use new Augment API
          console.log('💰 Fetching balance from Augment API...');
          await actions.fetchAugmentCredits();
          console.log('✅ Balance fetched from Augment API');

          // Also fetch subscription info
          try {
            await actions.fetchSubscription();
          } catch (e) {
            console.warn('⚠️ Failed to fetch subscription:', e);
          }

          // Fetch analytics from Augment API (daily, model, activity usage)
          try {
            console.log('📊 Fetching Augment analytics...');
            await actions.fetchAugmentAnalytics(30);
            console.log('✅ Augment analytics fetched');
          } catch (e) {
            console.warn('⚠️ Failed to fetch analytics, using defaults:', e);
            actions.setDefaultAnalytics();
          }
        } else if (useFreshData) {
          // Legacy: Fetch fresh data from Orb API
          console.log('💰 Fetching fresh balance from Orb API...');
          await actions.fetchCurrentBalance();
          console.log('✅ Fresh balance fetched from Orb API');
        } else {
          // Legacy: Fetch initial data from database
          console.log('💰 Fetching initial balance from database...');
          await actions.fetchInitialBalance();
          console.log('✅ Initial balance fetched from database');
        }
      } catch (error) {
        console.warn('⚠️ Failed to fetch balance, continuing anyway:', error);
      }

      // Fetch legacy analytics if not using Augment API
      if (!authStatusData.is_augment_configured) {
        try {
          console.log('📊 Fetching legacy usage analytics...');
          await actions.fetchUsageAnalytics();
          console.log('✅ Usage analytics fetched');
        } catch (error) {
          console.warn('⚠️ Failed to fetch usage analytics, continuing anyway:', error);
        }
      }

      try {
        console.log('🎧 Setting up backend event listeners...');
        await setupBackendEventListeners();
        console.log('✅ Event listeners set up');
      } catch (error) {
        console.warn('⚠️ Failed to set up event listeners, continuing anyway:', error);
      }

      console.log('🔄 Starting auto-refresh...');
      startAutoRefresh(60);
      console.log('✅ Auto-refresh started');
    } else {
      console.log('⚠️ Not authenticated, skipping balance fetching');
      connectionStatus.set('disconnected');
    }

    console.log('🎉 App initialized successfully');
    return true;
  } catch (error) {
    console.error('❌ Failed to initialize app:', error);
    console.error('Error details:', error.stack);

    console.log('⚠️ Initialization failed, but allowing UI to load anyway');
    connectionStatus.set('error');
    return true;
  }
}

// Store the unlisten function to prevent garbage collection
let unlistenBalanceUpdated = null;

// Set up event listeners for backend events
async function setupBackendEventListeners() {
  try {
    console.log('Setting up backend event listeners...');

    // Listen for balance updates from backend monitoring
    unlistenBalanceUpdated = await listen('balance-updated', (event) => {
      console.log('🎉 Received balance update from backend:', event.payload);
      currentBalance.set(event.payload);
      lastUpdate.set(new Date());
      connectionStatus.set('connected');
    });

    console.log('✅ Backend event listeners set up successfully');
  } catch (error) {
    console.error('❌ Failed to set up backend event listeners:', error);
    throw error;
  }
}

// Clean up event listeners
export function cleanupEventListeners() {
  if (unlistenBalanceUpdated) {
    unlistenBalanceUpdated();
    unlistenBalanceUpdated = null;
  }
}
