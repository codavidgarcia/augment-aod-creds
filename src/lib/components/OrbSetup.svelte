<script>
  import { createEventDispatcher } from 'svelte';
  import { orbConfig, actions } from '../stores/app.js';
  import { ExternalLink, AlertCircle, CheckCircle, Loader, Zap } from 'lucide-svelte';

  const dispatch = createEventDispatcher();

  let urlInput = '';
  let isLoading = false;
  let error = null;
  let success = false;
  let statusMessage = '';

  async function handleSubmit() {
    if (!urlInput.trim()) {
      error = 'Please enter a valid Augment Portal Endpoint';
      return;
    }

    isLoading = true;
    error = null;
    success = false;
    statusMessage = 'Parsing endpoint and fetching your current balance...';

    try {
      await actions.parseOrbUrl(urlInput.trim());
      success = true;
      statusMessage = 'Success! Your credits are now being displayed.';

      // Wait a moment to show success, then emit completion
      setTimeout(() => {
        dispatch('setup-complete');
      }, 1500);
    } catch (err) {
      error = err.message || 'Failed to parse endpoint. Please check the format and try again.';
      statusMessage = '';
    } finally {
      isLoading = false;
    }
  }

  function handlePaste() {
    // Clear any previous errors when user starts typing
    if (error) {
      error = null;
    }
  }
</script>

<!-- Use consistent background and layout with main app -->
<div class="min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center p-4">
  <div class="w-full max-w-2xl">
    <!-- Main Setup Card -->
    <div class="card p-8">
      <!-- Header with consistent branding -->
      <div class="text-center mb-8">
        <div class="flex justify-center mb-4">
          <div class="w-16 h-16 bg-primary-100 dark:bg-primary-900/30 rounded-full flex items-center justify-center">
            <Zap size="32" class="text-primary-600 dark:text-primary-400" />
          </div>
        </div>
        <h1 class="text-3xl font-bold text-gray-900 dark:text-gray-100 mb-2">
          Welcome to Augment Credit Monitor
        </h1>
        <p class="text-gray-600 dark:text-gray-400 text-lg">
          Connect your Augment Portal to start monitoring your credit balance in real-time.
        </p>
      </div>

      <!-- Instructions Card -->
      <div class="card-secondary p-6 mb-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4 flex items-center">
          <ExternalLink size="20" class="mr-2 text-primary-600 dark:text-primary-400" />
          How to get your Augment Portal Endpoint
        </h2>
        <ol class="space-y-2 text-gray-600 dark:text-gray-400">
          <li class="flex items-start">
            <span class="flex-shrink-0 w-6 h-6 bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full flex items-center justify-center text-sm font-medium mr-3 mt-0.5">1</span>
            Log in to your Augment account
          </li>
          <li class="flex items-start">
            <span class="flex-shrink-0 w-6 h-6 bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full flex items-center justify-center text-sm font-medium mr-3 mt-0.5">2</span>
            Navigate to your customer portal or billing page
          </li>
          <li class="flex items-start">
            <span class="flex-shrink-0 w-6 h-6 bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full flex items-center justify-center text-sm font-medium mr-3 mt-0.5">3</span>
            Look for the ledger summary or balance API endpoint
          </li>
          <li class="flex items-start">
            <span class="flex-shrink-0 w-6 h-6 bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full flex items-center justify-center text-sm font-medium mr-3 mt-0.5">4</span>
            Copy the complete endpoint URL including the token parameter
          </li>
        </ol>
      </div>

      <!-- URL Input Form -->
      <form on:submit|preventDefault={handleSubmit} class="space-y-6">
        <div class="form-group">
          <label for="url-input" class="form-label">
            Augment Portal Endpoint
          </label>
          <textarea
            id="url-input"
            bind:value={urlInput}
            on:input={handlePaste}
            placeholder="paste your endpoint URL here"
            class="input w-full font-mono text-sm"
            rows="3"
            disabled={isLoading || success}
          ></textarea>
          <p class="form-help">
            Paste the complete endpoint URL from your Augment portal, including authentication tokens
          </p>
        </div>

        <!-- Error Message -->
        {#if error}
          <div class="message-error">
            <AlertCircle size="16" />
            <span>{error}</span>
          </div>
        {/if}

        <!-- Status Message -->
        {#if statusMessage}
          <div class="message-info">
            <AlertCircle size="16" />
            <span>{statusMessage}</span>
          </div>
        {/if}

        <!-- Success Message -->
        {#if success}
          <div class="message-success">
            <CheckCircle size="16" />
            <span>Configuration saved successfully! Loading your dashboard...</span>
          </div>
        {/if}

        <!-- Submit Button -->
        <button
          type="submit"
          class="btn btn-primary w-full"
          disabled={isLoading || success || !urlInput.trim()}
        >
          {#if isLoading}
            <Loader size="16" class="animate-spin" />
            <span>Connecting & Fetching Balance...</span>
          {:else if success}
            <CheckCircle size="16" />
            <span>Setup Complete</span>
          {:else}
            <span>Connect to Augment Portal</span>
          {/if}
        </button>
      </form>

      <!-- Security Note -->
      <div class="message-info mt-6">
        <AlertCircle size="16" />
        <div>
          <p class="font-medium">Security & Privacy</p>
          <p class="text-sm opacity-90 mt-1">
            Your credentials are stored locally and encrypted. We only communicate with Augment's servers to fetch your balance data.
          </p>
        </div>
      </div>
    </div>
  </div>
</div>

<!-- All styling now uses the consistent design system classes -->
