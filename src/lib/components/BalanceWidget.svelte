<script>
  import { onMount } from 'svelte';
  import {
    currentBalance,
    balanceStatus,
    balanceStatusColor,
    formattedBalance,
    usageRate,
    timeRemaining,
    isLoading,
    lastUpdate,
    connectionStatus,
    authStatus,
    actions
  } from '../stores/app.js';
  import { RefreshCw, TrendingDown, TrendingUp, Clock, Zap } from 'lucide-svelte';

  export let compact = true;
  export const showDetails = false;

  let refreshing = false;

  async function handleRefresh() {
    if (refreshing) return;

    refreshing = true;
    try {
      // Use Augment API if configured, otherwise legacy
      if ($authStatus.is_augment_configured) {
        await actions.fetchAugmentCredits();
        await actions.fetchAugmentAnalytics(30);
      } else {
        await actions.fetchCurrentBalance();
        await actions.fetchUsageAnalytics();
      }
    } catch (error) {
      console.error('❌ Refresh failed:', error);
    } finally {
      refreshing = false;
    }
  }

  function formatLastUpdate(date) {
    if (!date) return 'Never';
    
    const now = new Date();
    const diff = now - date;
    const minutes = Math.floor(diff / 60000);
    
    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  // Remove duplicate initialization - this is now handled by the main page
</script>

<div class="balance-widget" class:compact>
  <!-- Compact View (Menu Bar Style) -->
  {#if compact}
    <div class="card flex items-center space-x-3 px-4 py-3">
      <!-- Status Indicator -->
      <div class="flex items-center space-x-2">
        <div
          class="w-3 h-3 rounded-full transition-colors duration-200 shadow-sm"
          class:bg-success-500={$balanceStatus === 'healthy'}
          class:bg-warning-500={$balanceStatus === 'warning'}
          class:bg-danger-500={$balanceStatus === 'critical'}
          class:bg-gray-400={$balanceStatus === 'unknown'}
          class:animate-pulse={$isLoading}
        ></div>

        <!-- Balance Display -->
        <span class="text-sm font-mono font-bold {$balanceStatusColor}">
          {$formattedBalance}
        </span>
      </div>

      <!-- Usage Rate (if available) -->
      {#if $usageRate && $usageRate.perHour > 0}
        <div class="flex items-center space-x-1 text-xs text-gray-600 dark:text-gray-400">
          <TrendingDown size="12" />
          <span>{actions.formatUsageRate($usageRate.perHour)}/h</span>
        </div>
      {/if}

      <!-- Time Remaining (if available) -->
      {#if $timeRemaining && $timeRemaining.hours}
        <div class="flex items-center space-x-1 text-xs text-gray-600 dark:text-gray-400">
          <Clock size="12" />
          <span>{actions.formatTimeRemaining($timeRemaining.hours)}</span>
        </div>
      {/if}

      <!-- Refresh Button -->
      <button
        on:click={handleRefresh}
        disabled={refreshing || $isLoading}
        class="btn btn-outline btn-sm p-1.5 rounded-lg transition-all duration-200 disabled:opacity-50 hover:scale-105"
        title="Refresh balance"
        aria-label="Refresh balance"
      >
        <RefreshCw
          size="14"
          class="text-gray-600 dark:text-gray-400 transition-transform duration-200 {refreshing || $isLoading ? 'animate-spin' : ''}"
        />
      </button>
    </div>
  {:else}
    <!-- Detailed View -->
    <div class="card-elevated p-8 space-y-6">
      <!-- Header -->
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Credit Balance
        </h2>
        
        <div class="flex items-center space-x-2">
          <!-- Connection Status -->
          <div class="flex items-center space-x-1">
            <div 
              class="w-2 h-2 rounded-full"
              class:bg-success-500={$connectionStatus === 'connected'}
              class:bg-warning-500={$connectionStatus === 'disconnected'}
              class:bg-danger-500={$connectionStatus === 'error'}
            ></div>
            <span class="text-xs text-gray-500 dark:text-gray-400 capitalize">
              {$connectionStatus}
            </span>
          </div>

          <!-- Refresh Button -->
          <button
            on:click={handleRefresh}
            disabled={refreshing || $isLoading}
            class="btn btn-primary btn-sm"
            aria-label="Refresh balance"
          >
            <RefreshCw
              size="14"
              class="mr-1 transition-transform duration-200 {refreshing || $isLoading ? 'animate-spin' : ''}"
            />
            Refresh
          </button>
        </div>
      </div>

      <!-- Main Balance Display -->
      <div class="text-center space-y-2">
        <div class="text-4xl font-mono font-bold {$balanceStatusColor}">
          {$formattedBalance}
        </div>
        <div class="text-sm text-gray-600 dark:text-gray-400">
          Credits Remaining
        </div>
        
        <!-- Status Badge -->
        <div class="flex justify-center">
          <span 
            class="badge"
            class:badge-success={$balanceStatus === 'healthy'}
            class:badge-warning={$balanceStatus === 'warning'}
            class:badge-danger={$balanceStatus === 'critical'}
            class:badge-info={$balanceStatus === 'unknown'}
          >
            {actions.getBalanceIcon($balanceStatus)} 
            {$balanceStatus === 'unknown' ? 'Unknown' : $balanceStatus.charAt(0).toUpperCase() + $balanceStatus.slice(1)}
          </span>
        </div>
      </div>

      <!-- Usage Statistics -->
      {#if $usageRate}
        <div class="grid grid-cols-2 gap-4">
          <div class="text-center p-3 bg-gray-50 dark:bg-gray-700 rounded-lg">
            <div class="flex items-center justify-center space-x-1 text-sm text-gray-600 dark:text-gray-400 mb-1">
              <Zap size="14" />
              <span>Usage Rate</span>
            </div>
            <div class="text-lg font-semibold text-gray-900 dark:text-gray-100">
              {actions.formatUsageRate($usageRate.perHour)}/hour
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-500">
              {actions.formatUsageRate($usageRate.perDay)}/day
            </div>
          </div>

          {#if $timeRemaining && $timeRemaining.hours}
            <div class="text-center p-3 bg-gray-50 dark:bg-gray-700 rounded-lg">
              <div class="flex items-center justify-center space-x-1 text-sm text-gray-600 dark:text-gray-400 mb-1">
                <Clock size="14" />
                <span>Time Left</span>
              </div>
              <div class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                {actions.formatTimeRemaining($timeRemaining.hours)}
              </div>
              {#if $timeRemaining.days}
                <div class="text-xs text-gray-500 dark:text-gray-500">
                  ({$timeRemaining.days.toFixed(1)} days)
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Last Update -->
      {#if $lastUpdate}
        <div class="text-center text-xs text-gray-500 dark:text-gray-500">
          Last updated: {formatLastUpdate($lastUpdate)}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .balance-widget.compact {
    @apply min-w-0;
  }
  
  .balance-widget:not(.compact) {
    @apply w-full max-w-md;
  }
</style>
