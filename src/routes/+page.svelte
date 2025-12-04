<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import BalanceWidget from '../lib/components/BalanceWidget.svelte';
  import UsageChart from '../lib/components/UsageChart.svelte';
  import OrbSetup from '../lib/components/OrbSetup.svelte';
  import AugmentLogin from '../lib/components/AugmentLogin.svelte';
  import {
    currentBalance,
    usageAnalytics,
    consumptionHistory,
    appConfig,
    balanceStatus,
    orbConfig,
    authStatus,
    subscriptionInfo,
    actions,
    initializeApp,
    stopAutoRefresh,
    cleanupEventListeners,
    connectionStatus,
    lastUpdate,
    formattedBalance,
    isLoading
  } from '../lib/stores/app.js';
  import { Settings, BarChart3, TrendingUp, Clock, Zap, Bell, Minimize2, Maximize2, Eye, EyeOff, CheckCircle, AlertCircle, ExternalLink, Wifi, WifiOff, AlertTriangle, LogOut, Menu, X } from 'lucide-svelte';

  let activeTab = 'overview';
  let showSettings = false;
  let configFormInitialized = false;
  let showSetup = false;
  let showMobileMenu = false;

  // Custom confirmation dialog state
  let showCustomConfirm = false;
  let confirmMessage = '';
  let confirmCallback = null;

  // Configuration form
  let configForm = {
    orb_token: '',
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
  };

  // Only update form when config is first loaded or explicitly reset
  $: if ($appConfig && !configFormInitialized) {
    configForm = { ...$appConfig };
    configFormInitialized = true;
  }

  async function saveConfig() {
    try {
      await actions.updateConfig(configForm);
      showSettings = false;
    } catch (error) {
      console.error('Failed to save config:', error);
      alert('Failed to save configuration: ' + error.message);
    }
  }

  // Production-ready functions only

  // Logout functionality with custom confirmation dialog
  function showCustomConfirmDialog(message, callback) {
    confirmMessage = message;
    confirmCallback = callback;
    showCustomConfirm = true;
  }

  function handleCustomConfirmYes() {
    showCustomConfirm = false;
    if (confirmCallback) {
      confirmCallback(true);
    }
  }

  function handleCustomConfirmNo() {
    showCustomConfirm = false;
    if (confirmCallback) {
      confirmCallback(false);
    }
  }

  // Clean logout functionality
  async function handleLogout() {
    showCustomConfirmDialog(
      'Are you sure you want to logout? This will clear your credentials and return you to the setup screen.',
      async (confirmed) => {
        if (confirmed) {
          try {
            console.log('🚪 Logging out user...');

            // Stop auto-refresh to prevent background operations
            stopAutoRefresh();

            // Clean up event listeners
            cleanupEventListeners();

            // Clear both Augment session and legacy Orb config
            try {
              await actions.clearAugmentSession();
            } catch (e) {
              console.warn('Failed to clear Augment session:', e);
            }
            try {
              await actions.clearOrbConfig();
            } catch (e) {
              console.warn('Failed to clear Orb config:', e);
            }

            // Refresh the configuration to update UI state
            await actions.fetchAuthStatus();
            await actions.fetchOrbConfig();

            // Reset local component state
            showSettings = false;
            configFormInitialized = false;

            // Reset configuration form to defaults
            configForm = {
              orb_token: '',
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
            };

            // Show the setup screen
            showSetup = true;

            console.log('✅ Logout completed successfully - user returned to setup screen');
          } catch (error) {
            console.error('❌ Logout failed:', error);
            alert('Failed to logout: ' + error.message);
          }
        }
      }
    );
  }

  function resetConfig() {
    configForm = { ...$appConfig };
    configFormInitialized = true; // Ensure form stays reset
  }

  // Window management functions
  async function hideToBackground() {
    try {
      await actions.hideWindow();
    } catch (error) {
      console.error('Failed to hide window:', error);
    }
  }

  async function showWindow() {
    try {
      await actions.showWindow();
    } catch (error) {
      console.error('Failed to show window:', error);
    }
  }

  async function toggleWindow() {
    try {
      await actions.toggleWindow();
    } catch (error) {
      console.error('Failed to toggle window:', error);
    }
  }

  function handleSetupComplete() {
    showSetup = false;
    // Re-initialize the app with fresh data since we just configured new credentials
    initializeApp(true); // true = use fresh data from API
  }

  function handleLoginComplete(event) {
    showSetup = false;
    // Re-initialize the app with fresh data
    initializeApp(true);
  }

  onMount(async () => {
    // App is already initialized by layout, just set up subscriptions
    // Check if we need to show setup based on auth status
    authStatus.subscribe(status => {
      // Show setup if not authenticated via new Augment API
      if (!status.is_authenticated) {
        // Also check legacy Orb config
        orbConfig.subscribe(config => {
          showSetup = !config.is_configured;
        });
      } else {
        showSetup = false;
      }
    });
  });

  onDestroy(() => {
    // Clean up auto-refresh and event listeners when component is destroyed
    stopAutoRefresh();
    cleanupEventListeners();
  });
</script>

<svelte:head>
  <title>Augment Credit Monitor</title>
</svelte:head>

{#if showSetup}
  <!-- Show new Augment login by default, with option for legacy Orb setup -->
  <AugmentLogin on:login-complete={handleLoginComplete} />
{:else}
<!-- Enhanced Header with Glass Morphism and Status Indicators -->
<div class="header-container">
  <div class="header-content">
    <div class="flex items-center justify-between">
      <!-- Left Section: Branding & Status -->
      <div class="flex items-center space-x-4">
        <!-- App Branding -->
        <div class="flex-shrink-0">
          <h1 class="header-title">
            Augment Credit Monitor
          </h1>
          <div class="flex items-center space-x-3 mt-1">
            <p class="header-subtitle">
              Real-time credit balance monitoring and analytics
            </p>
            <!-- Connection Status Indicator -->
            <div class="hidden sm:flex">
              {#if $connectionStatus === 'connected'}
                <div class="status-connected">
                  <Wifi size="12" />
                  <span>Connected</span>
                </div>
              {:else if $connectionStatus === 'disconnected'}
                <div class="status-disconnected">
                  <WifiOff size="12" />
                  <span>Disconnected</span>
                </div>
              {:else}
                <div class="status-error">
                  <AlertTriangle size="12" />
                  <span>Error</span>
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- Quick Balance Display (Desktop) -->
        <div class="hidden lg:flex items-center space-x-3 ml-8">
          <div class="flex items-center space-x-2">
            <div
              class="w-3 h-3 rounded-full transition-all duration-300 shadow-sm"
              class:bg-success-500={$balanceStatus === 'healthy'}
              class:bg-warning-500={$balanceStatus === 'warning'}
              class:bg-danger-500={$balanceStatus === 'critical'}
              class:bg-gray-400={$balanceStatus === 'unknown'}
              class:animate-pulse={$isLoading}
            ></div>
            <span class="text-lg font-mono font-bold text-gray-900 dark:text-gray-100">
              {$formattedBalance}
            </span>
            <span class="text-sm text-gray-500 dark:text-gray-400">credits</span>
          </div>
          {#if $lastUpdate}
            <div class="text-xs text-gray-400 dark:text-gray-500">
              Updated {new Date($lastUpdate).toLocaleTimeString()}
            </div>
          {/if}
        </div>
      </div>

      <!-- Right Section: Actions -->
      <div class="flex items-center space-x-2">
        <!-- Desktop Actions -->
        <div class="hidden md:flex items-center space-x-2">
          <!-- Window Management Group -->
          <div class="btn-group-compact">
            <button
              class="btn btn-sm btn-ghost"
              on:click={hideToBackground}
              title="Hide to Background - Continue monitoring in system tray"
              aria-label="Hide to Background"
            >
              <EyeOff size="16" />
            </button>

            <button
              class="btn btn-sm btn-ghost"
              on:click={toggleWindow}
              title="Toggle window visibility"
              aria-label="Toggle Window"
            >
              <Minimize2 size="16" />
            </button>
          </div>

          <!-- Divider -->
          <div class="w-px h-6 bg-gray-300 dark:bg-gray-600"></div>

          <!-- Primary Actions -->
          <div class="btn-group">
            <button
              class="btn btn-sm btn-danger-outline"
              on:click={handleLogout}
              title="Logout and return to setup screen"
              aria-label="Logout"
            >
              <LogOut size="16" class="mr-1" />
              <span class="hidden lg:inline">Logout</span>
            </button>

            <button
              class="btn btn-sm btn-primary"
              on:click={() => showSettings = !showSettings}
              aria-label="Settings"
            >
              <Settings size="16" class="mr-1" />
              <span class="hidden lg:inline">Settings</span>
            </button>
          </div>
        </div>

        <!-- Mobile Menu Button -->
        <div class="md:hidden">
          <button
            class="btn btn-sm btn-ghost"
            on:click={() => showMobileMenu = !showMobileMenu}
            aria-label="Toggle mobile menu"
          >
            {#if showMobileMenu}
              <X size="20" />
            {:else}
              <Menu size="20" />
            {/if}
          </button>
        </div>
      </div>
    </div>

    <!-- Mobile Menu -->
    {#if showMobileMenu}
      <div class="md:hidden mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 animate-fade-in">
        <!-- Mobile Balance Display -->
        <div class="flex items-center justify-between mb-4 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
          <div class="flex items-center space-x-2">
            <div
              class="w-3 h-3 rounded-full transition-all duration-300"
              class:bg-success-500={$balanceStatus === 'healthy'}
              class:bg-warning-500={$balanceStatus === 'warning'}
              class:bg-danger-500={$balanceStatus === 'critical'}
              class:bg-gray-400={$balanceStatus === 'unknown'}
              class:animate-pulse={$isLoading}
            ></div>
            <span class="text-lg font-mono font-bold text-gray-900 dark:text-gray-100">
              {$formattedBalance}
            </span>
            <span class="text-sm text-gray-500 dark:text-gray-400">credits</span>
          </div>
          <!-- Connection Status (Mobile) -->
          {#if $connectionStatus === 'connected'}
            <div class="status-connected">
              <Wifi size="12" />
              <span>Connected</span>
            </div>
          {:else if $connectionStatus === 'disconnected'}
            <div class="status-disconnected">
              <WifiOff size="12" />
              <span>Disconnected</span>
            </div>
          {:else}
            <div class="status-error">
              <AlertTriangle size="12" />
              <span>Error</span>
            </div>
          {/if}
        </div>

        <!-- Mobile Action Buttons -->
        <div class="grid grid-cols-2 gap-2">
          <button
            class="btn btn-sm btn-outline"
            on:click={() => {
              hideToBackground();
              showMobileMenu = false;
            }}
          >
            <EyeOff size="16" class="mr-2" />
            Hide
          </button>

          <button
            class="btn btn-sm btn-outline"
            on:click={() => {
              toggleWindow();
              showMobileMenu = false;
            }}
          >
            <Minimize2 size="16" class="mr-2" />
            Toggle
          </button>

          <button
            class="btn btn-sm btn-primary"
            on:click={() => {
              showSettings = !showSettings;
              showMobileMenu = false;
            }}
          >
            <Settings size="16" class="mr-2" />
            Settings
          </button>

          <button
            class="btn btn-sm btn-danger-outline"
            on:click={() => {
              handleLogout();
              showMobileMenu = false;
            }}
          >
            <LogOut size="16" class="mr-2" />
            Logout
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- Main Content Container -->
<div class="container mx-auto px-4 sm:px-6 lg:px-8 py-6 max-w-6xl">

  {#if showSettings}
    <!-- Settings Panel -->
    <div class="card-elevated p-6 mb-8">
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Configuration
        </h2>
        <div class="flex space-x-2">
          <button class="btn btn-outline btn-sm" on:click={resetConfig}>
            Reset
          </button>
          <button class="btn btn-primary btn-sm" on:click={saveConfig}>
            Save
          </button>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- Orb Configuration -->
        <div class="md:col-span-2">
          <h3 class="text-md font-medium text-gray-900 dark:text-gray-100 mb-4">
            Orb Portal Configuration
          </h3>

          {#if $orbConfig.is_configured}
            <div class="message-success p-4 mb-4">
              <CheckCircle size="16" />
              <div class="flex-1">
                <div class="font-medium mb-1">Orb Portal Connected</div>
                <div class="text-xs opacity-90 space-y-1">
                  <div><strong>Customer ID:</strong> {$orbConfig.customer_id}</div>
                  <div><strong>Pricing Unit ID:</strong> {$orbConfig.pricing_unit_id}</div>
                  <div><strong>Token:</strong> {$orbConfig.has_token ? '••••••••••••••••' : 'Not set'}</div>
                </div>
              </div>
            </div>

            <div class="flex gap-2">
              <button
                class="btn btn-outline btn-sm text-red-600 border-red-300 hover:bg-red-50 dark:text-red-400 dark:border-red-700 dark:hover:bg-red-900/20"
                on:click={handleLogout}
                type="button"
              >
                <ExternalLink size="14" class="mr-1" />
                Logout
              </button>
            </div>
          {:else}
            <div class="message-warning p-4 mb-4">
              <AlertCircle size="16" />
              <div class="flex-1">
                <div class="font-medium mb-1">Augment Portal Not Configured</div>
                <p class="text-xs opacity-90 mb-3">
                  You need to configure your Augment Portal Endpoint to monitor your credit balance.
                </p>
                <button
                  class="btn btn-primary btn-sm"
                  on:click={() => showSetup = true}
                >
                  Configure Augment Portal
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Polling Interval -->
        <div class="form-group">
          <label for="polling_interval" class="form-label">
            Polling Interval (seconds)
          </label>
          <input
            id="polling_interval"
            type="number"
            min="30"
            max="3600"
            bind:value={configForm.polling_interval_seconds}
            class="input"
          />
          <div class="form-help">How often to check your balance (30-3600 seconds)</div>
        </div>

        <!-- Low Balance Threshold -->
        <div class="form-group">
          <label for="low_threshold" class="form-label">
            Low Balance Threshold
          </label>
          <input
            id="low_threshold"
            type="number"
            min="0"
            bind:value={configForm.low_balance_threshold}
            class="input"
          />
          <div class="form-help">Show warning when balance drops below this amount</div>
        </div>

        <!-- Critical Balance Threshold -->
        <div class="form-group">
          <label for="critical_threshold" class="form-label">
            Critical Balance Threshold
          </label>
          <input
            id="critical_threshold"
            type="number"
            min="0"
            bind:value={configForm.critical_balance_threshold}
            class="input"
          />
          <div class="form-help">Show critical alert when balance drops below this amount</div>
        </div>

        <!-- Data Retention -->
        <div class="form-group">
          <label for="data_retention" class="form-label">
            Data Retention (days)
          </label>
          <input
            id="data_retention"
            type="number"
            min="1"
            max="365"
            bind:value={configForm.data_retention_days}
            class="input"
          />
          <div class="form-help">How long to keep historical data (1-365 days)</div>
        </div>

        <!-- Checkboxes -->
        <div class="md:col-span-2 space-y-4">
          <div class="form-group">
            <label class="flex items-center cursor-pointer">
              <input
                type="checkbox"
                bind:checked={configForm.enable_notifications}
                class="rounded border-gray-300 text-primary-600 focus:ring-primary-500 focus:ring-offset-0"
              />
              <span class="ml-3 text-sm font-medium text-gray-700 dark:text-gray-300">Enable notifications</span>
            </label>
            <div class="form-help ml-6">Show system notifications for balance alerts</div>
          </div>

          <div class="form-group">
            <label class="flex items-center cursor-pointer">
              <input
                type="checkbox"
                bind:checked={configForm.enable_sound_alerts}
                class="rounded border-gray-300 text-primary-600 focus:ring-primary-500 focus:ring-offset-0"
              />
              <span class="ml-3 text-sm font-medium text-gray-700 dark:text-gray-300">Enable sound alerts</span>
            </label>
            <div class="form-help ml-6">Play sound when balance reaches critical levels</div>
          </div>

          <div class="form-group">
            <label class="flex items-center cursor-pointer">
              <input
                type="checkbox"
                bind:checked={configForm.auto_start}
                class="rounded border-gray-300 text-primary-600 focus:ring-primary-500 focus:ring-offset-0"
              />
              <span class="ml-3 text-sm font-medium text-gray-700 dark:text-gray-300">Start automatically with system</span>
            </label>
            <div class="form-help ml-6">Launch the app when your computer starts</div>
          </div>

          <div class="form-group">
            <label class="flex items-center cursor-pointer">
              <input
                type="checkbox"
                bind:checked={configForm.window_always_on_top}
                class="rounded border-gray-300 text-primary-600 focus:ring-primary-500 focus:ring-offset-0"
              />
              <span class="ml-3 text-sm font-medium text-gray-700 dark:text-gray-300">Keep window always on top</span>
            </label>
            <div class="form-help ml-6">Window stays above other applications</div>
          </div>

          <div class="form-group">
            <label class="flex items-center cursor-pointer">
              <input
                type="checkbox"
                bind:checked={configForm.compact_mode}
                class="rounded border-gray-300 text-primary-600 focus:ring-primary-500 focus:ring-offset-0"
              />
              <span class="ml-3 text-sm font-medium text-gray-700 dark:text-gray-300">Compact mode</span>
            </label>
            <div class="form-help ml-6">Use smaller interface elements</div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Main Content -->
  <div class="space-y-8">
    <!-- Balance Widget -->
    <div class="flex justify-center">
      <BalanceWidget compact={false} />
    </div>

    <!-- Navigation Tabs -->
    <div class="border-b border-gray-200 dark:border-gray-700">
      <nav class="-mb-px flex space-x-8">
        <button
          class="py-2 px-1 border-b-2 font-medium text-sm transition-colors duration-200"
          class:border-primary-500={activeTab === 'overview'}
          class:text-primary-600={activeTab === 'overview'}
          class:dark:text-primary-400={activeTab === 'overview'}
          class:border-transparent={activeTab !== 'overview'}
          class:text-gray-500={activeTab !== 'overview'}
          class:hover:text-gray-700={activeTab !== 'overview'}
          class:dark:text-gray-400={activeTab !== 'overview'}
          class:dark:hover:text-gray-300={activeTab !== 'overview'}
          on:click={() => activeTab = 'overview'}
        >
          <BarChart3 size="16" class="inline mr-2" />
          Overview
        </button>

        <button
          class="py-2 px-1 border-b-2 font-medium text-sm transition-colors duration-200"
          class:border-primary-500={activeTab === 'analytics'}
          class:text-primary-600={activeTab === 'analytics'}
          class:dark:text-primary-400={activeTab === 'analytics'}
          class:border-transparent={activeTab !== 'analytics'}
          class:text-gray-500={activeTab !== 'analytics'}
          class:hover:text-gray-700={activeTab !== 'analytics'}
          class:dark:text-gray-400={activeTab !== 'analytics'}
          class:dark:hover:text-gray-300={activeTab !== 'analytics'}
          on:click={() => activeTab = 'analytics'}
        >
          <TrendingUp size="16" class="inline mr-2" />
          Analytics
        </button>
      </nav>
    </div>

    <!-- Tab Content -->
    {#if activeTab === 'overview'}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
        <!-- Balance History Chart -->
        <div class="card-elevated p-6">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
            Balance History
          </h3>
          <UsageChart type="balance" height={250} />
        </div>

        <!-- Usage Rate Chart -->
        <div class="card-elevated p-6">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
            Usage Rate
          </h3>
          <UsageChart type="usage" height={250} />
        </div>
      </div>
    {:else if activeTab === 'analytics'}
      {#if $usageAnalytics}
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <!-- Usage Rate -->
          <div class="card-elevated p-6 text-center">
            <div class="flex items-center justify-center mb-3">
              <Zap size="24" class="text-primary-600 dark:text-primary-400" />
            </div>
            <div class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-1">
              {actions.formatUsageRate($usageAnalytics.usage_rate_per_hour)}
            </div>
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Credits per hour
            </div>
          </div>

          <!-- Time Remaining -->
          <div class="card-elevated p-6 text-center">
            <div class="flex items-center justify-center mb-3">
              <Clock size="24" class="text-warning-600 dark:text-warning-400" />
            </div>
            <div class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-1">
              {$usageAnalytics.estimated_hours_remaining ? actions.formatTimeRemaining($usageAnalytics.estimated_hours_remaining) : '--'}
            </div>
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Estimated remaining
            </div>
          </div>

          <!-- Efficiency Score -->
          <div class="card-elevated p-6 text-center">
            <div class="flex items-center justify-center mb-3">
              <TrendingUp size="24" class="text-success-600 dark:text-success-400" />
            </div>
            <div class="text-2xl font-bold text-gray-900 dark:text-gray-100">
              {$usageAnalytics.efficiency_score.toFixed(0)}%
            </div>
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Efficiency score
            </div>
          </div>

          <!-- Trend -->
          <div class="card p-6 text-center">
            <div class="flex items-center justify-center mb-2">
              <span class="text-xl">
                {actions.getTrendIcon($usageAnalytics.trend)}
              </span>
            </div>
            <div class="text-2xl font-bold text-gray-900 dark:text-gray-100">
              {$usageAnalytics.trend}
            </div>
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Usage trend
            </div>
          </div>
        </div>

        <!-- Detailed Analytics -->
        <div class="card p-6">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
            Detailed Analytics
          </h3>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <h4 class="font-medium text-gray-900 dark:text-gray-100 mb-2">Usage Statistics</h4>
              <dl class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <dt class="text-gray-600 dark:text-gray-400">Average daily usage:</dt>
                  <dd class="text-gray-900 dark:text-gray-100">{$usageAnalytics.average_session_usage.toFixed(0)} credits</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-gray-600 dark:text-gray-400">Daily usage rate:</dt>
                  <dd class="text-gray-900 dark:text-gray-100">{actions.formatUsageRate($usageAnalytics.usage_rate_per_day)} credits/day</dd>
                </div>
                {#if $usageAnalytics.peak_usage_hour !== null}
                  <div class="flex justify-between">
                    <dt class="text-gray-600 dark:text-gray-400">Peak usage hour:</dt>
                    <dd class="text-gray-900 dark:text-gray-100">{$usageAnalytics.peak_usage_hour}:00</dd>
                  </div>
                {/if}
              </dl>
            </div>

            <div>
              <h4 class="font-medium text-gray-900 dark:text-gray-100 mb-2">Projections</h4>
              <dl class="space-y-2 text-sm">
                {#if $usageAnalytics.estimated_days_remaining}
                  <div class="flex justify-between">
                    <dt class="text-gray-600 dark:text-gray-400">Days remaining:</dt>
                    <dd class="text-gray-900 dark:text-gray-100">{$usageAnalytics.estimated_days_remaining.toFixed(1)} days</dd>
                  </div>
                {/if}
                <div class="flex justify-between">
                  <dt class="text-gray-600 dark:text-gray-400">Analysis period:</dt>
                  <dd class="text-gray-900 dark:text-gray-100">{Math.round($usageAnalytics.total_usage_period / 24)} days</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-gray-600 dark:text-gray-400">Data points:</dt>
                  <dd class="text-gray-900 dark:text-gray-100">{$usageAnalytics.balance_history?.length || 0} records</dd>
                </div>
              </dl>
            </div>
          </div>
        </div>

        <!-- Usage Breakdown (Augment API with model and activity data) -->
        {#if $consumptionHistory && ($consumptionHistory.model_usage || $consumptionHistory.activity_usage)}
          <div class="card p-6 mt-6">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              Usage Breakdown (Last 30 Days)
            </h3>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
              <!-- By Model -->
              {#if $consumptionHistory.model_usage && $consumptionHistory.model_usage.length > 0}
                <div>
                  <h4 class="font-medium text-gray-900 dark:text-gray-100 mb-3">📊 By Model</h4>
                  <div class="space-y-2">
                    {#each $consumptionHistory.model_usage.sort((a, b) => b.credits - a.credits) as model}
                      <div class="flex justify-between items-center">
                        <span class="text-sm text-gray-600 dark:text-gray-400">
                          {model.model_name}
                        </span>
                        <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                          {model.credits.toLocaleString()}
                        </span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- By Activity Type -->
              {#if $consumptionHistory.activity_usage && $consumptionHistory.activity_usage.length > 0}
                <div>
                  <h4 class="font-medium text-gray-900 dark:text-gray-100 mb-3">🎯 By Activity</h4>
                  <div class="space-y-2">
                    {#each $consumptionHistory.activity_usage.sort((a, b) => b.credits - a.credits) as activity}
                      <div class="flex justify-between items-center">
                        <span class="text-sm text-gray-600 dark:text-gray-400">
                          {activity.activity_type}
                        </span>
                        <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                          {activity.credits.toLocaleString()}
                        </span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      {:else}
        <div class="card p-12 text-center">
          <div class="text-4xl mb-4">📊</div>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
            No Analytics Data
          </h3>
          <p class="text-gray-600 dark:text-gray-400">
            Analytics will be available after collecting usage data over time.
          </p>
        </div>
      {/if}
    {/if}
  </div>
</div> <!-- End Main Content Container -->
{/if}

<!-- Custom Confirmation Dialog -->
{#if showCustomConfirm}
  <div
    class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
    on:click={handleCustomConfirmNo}
    on:keydown={(e) => e.key === 'Escape' && handleCustomConfirmNo()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
  >
    <div
      class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full mx-4"
      on:click|stopPropagation
      on:keydown|stopPropagation
      role="document"
    >
      <div class="p-6">
        <div class="flex items-center mb-4">
          <div class="w-10 h-10 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center mr-3">
            <ExternalLink size="20" class="text-red-600 dark:text-red-400" />
          </div>
          <h3 id="confirm-title" class="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Confirm Logout
          </h3>
        </div>
        <p class="text-gray-600 dark:text-gray-400 mb-6 ml-13">
          {confirmMessage}
        </p>
        <div class="flex justify-end space-x-3">
          <button
            class="btn btn-outline"
            on:click={handleCustomConfirmNo}
            type="button"
          >
            Cancel
          </button>
          <button
            class="btn btn-outline text-red-600 border-red-300 hover:bg-red-50 dark:text-red-400 dark:border-red-700 dark:hover:bg-red-900/20"
            on:click={handleCustomConfirmYes}
            type="button"
          >
            Logout
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
