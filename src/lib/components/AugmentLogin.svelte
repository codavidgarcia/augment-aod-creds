<script>
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { authStatus, actions } from '../stores/app.js';
  import { ExternalLink, AlertCircle, CheckCircle, Loader, Zap, Cookie, Key, LogIn, Clipboard } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  const dispatch = createEventDispatcher();

  let sessionCookie = '';
  let isLoading = false;
  let isWebViewLoading = false;
  let error = null;
  let success = false;
  let statusMessage = '';
  let showManualInput = false;
  let showPastePrompt = false;
  let unlistenLoginComplete = null;
  let clipboardCheckInterval = null;

  let unlistenLoginSuccess = null;
  let unlistenLoginError = null;

  onMount(async () => {
    // Listen for login-complete event from the WebView (legacy)
    unlistenLoginComplete = await listen('login-complete', (event) => {
      console.log('Login complete event received:', event.payload);
      success = true;
      isWebViewLoading = false;
      showPastePrompt = false;
      statusMessage = `Connected as ${event.payload.email}! Balance: ${event.payload.balance.toLocaleString()} credits`;

      setTimeout(() => {
        dispatch('login-complete', event.payload);
      }, 1500);
    });

    // Listen for automatic login success from URL callback
    unlistenLoginSuccess = await listen('login-success', async () => {
      console.log('🎉 Login success event received!');
      success = true;
      isWebViewLoading = false;
      showPastePrompt = false;
      statusMessage = 'Connected successfully! Loading your data...';

      // Fetch the latest auth status and data
      try {
        console.log('🔄 Fetching auth status after login...');
        const authData = await actions.fetchAuthStatus();
        console.log('📋 Auth status after login:', authData);

        statusMessage = `Connected as ${authData.user_email || 'user'}!`;

        // Dispatch login-complete to parent immediately
        console.log('📤 Dispatching login-complete event');
        dispatch('login-complete', { email: authData.user_email });
      } catch (err) {
        console.error('❌ Failed to refresh after login:', err);
        // Still dispatch login-complete even on error
        dispatch('login-complete', {});
      }
    });

    // Listen for login errors
    unlistenLoginError = await listen('login-error', (event) => {
      console.log('Login error event received:', event.payload);
      error = event.payload || 'Login failed. Please try again.';
      isWebViewLoading = false;
      showPastePrompt = false;
    });
  });

  onDestroy(() => {
    if (unlistenLoginComplete) unlistenLoginComplete();
    if (unlistenLoginSuccess) unlistenLoginSuccess();
    if (unlistenLoginError) unlistenLoginError();
    if (clipboardCheckInterval) clearInterval(clipboardCheckInterval);
  });

  async function openLoginWebView() {
    isWebViewLoading = true;
    showPastePrompt = false;
    error = null;
    statusMessage = 'Opening Augment login window...';

    try {
      await invoke('open_augment_login');
      statusMessage = 'Please log in to Augment in the popup window...';

      // After a delay, show the paste prompt as fallback
      setTimeout(() => {
        if (isWebViewLoading && !success) {
          showPastePrompt = true;
          statusMessage = 'Log in and click "Connect to App" in the popup window';
        }
      }, 8000);
    } catch (err) {
      error = err.message || 'Failed to open login window';
      statusMessage = '';
      isWebViewLoading = false;
    }
  }

  async function pasteFromClipboard() {
    try {
      const clipboardText = await navigator.clipboard.readText();
      if (clipboardText && clipboardText.length > 100) {
        sessionCookie = clipboardText;
        await handleManualSubmit();
      } else {
        error = 'No valid session cookie found in clipboard. Please copy it from the login window first.';
      }
    } catch (err) {
      error = 'Could not read clipboard. Please paste manually below.';
      showManualInput = true;
    }
  }

  async function handleManualSubmit() {
    if (!sessionCookie.trim()) {
      error = 'Please enter your session cookie';
      return;
    }

    isLoading = true;
    isWebViewLoading = false;
    showPastePrompt = false;
    error = null;
    success = false;
    statusMessage = 'Validating session and fetching your credits...';

    try {
      const result = await actions.saveSessionCookie(sessionCookie.trim());
      success = true;
      statusMessage = `Connected as ${result.email}! Balance: ${result.balance.toLocaleString()} credits`;

      setTimeout(() => {
        dispatch('login-complete', result);
      }, 1500);
    } catch (err) {
      error = err.message || 'Failed to validate session. The cookie may have expired.';
      statusMessage = '';
    } finally {
      isLoading = false;
    }
  }

  function openAugmentPortal() {
    // Open the Augment app in the default browser
    window.open('https://app.augmentcode.com/account/subscription', '_blank');
  }
</script>

<div class="min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center p-4">
  <div class="w-full max-w-2xl">
    <div class="card p-8">
      <!-- Header -->
      <div class="text-center mb-8">
        <div class="flex justify-center mb-4">
          <div class="w-16 h-16 bg-primary-100 dark:bg-primary-900/30 rounded-full flex items-center justify-center">
            <Zap size="32" class="text-primary-600 dark:text-primary-400" />
          </div>
        </div>
        <h1 class="text-3xl font-bold text-gray-900 dark:text-gray-100 mb-2">
          Connect to Augment
        </h1>
        <p class="text-gray-600 dark:text-gray-400 text-lg">
          Monitor your Augment credit balance in real-time
        </p>
      </div>

      <!-- Primary Login Button -->
      <div class="space-y-4 mb-6">
        <button
          on:click={openLoginWebView}
          class="btn btn-primary w-full py-4 text-lg"
          disabled={isWebViewLoading || isLoading || success}
        >
          {#if isWebViewLoading}
            <Loader size="20" class="animate-spin" />
            <span>Waiting for login...</span>
          {:else if success}
            <CheckCircle size="20" />
            <span>Connected!</span>
          {:else}
            <LogIn size="20" />
            <span>Login with Augment</span>
          {/if}
        </button>

        <p class="text-center text-sm text-gray-500 dark:text-gray-400">
          A login window will open. Sign in with your Augment account.
        </p>
      </div>

      {#if error}
        <div class="message-error mb-4">
          <AlertCircle size="16" />
          <span>{error}</span>
        </div>
      {/if}

      {#if statusMessage && !error}
        <div class={success ? "message-success mb-4" : "message-info mb-4"}>
          {#if success}<CheckCircle size="16" />{:else}<Loader size="16" class="animate-spin" />{/if}
          <span>{statusMessage}</span>
        </div>
      {/if}

      <!-- Paste Session Button (shown after WebView login) -->
      {#if showPastePrompt && !success}
        <div class="space-y-3 mb-6">
          <button
            on:click={pasteFromClipboard}
            class="btn btn-secondary w-full py-3"
            disabled={isLoading}
          >
            {#if isLoading}
              <Loader size="18" class="animate-spin" />
              <span>Connecting...</span>
            {:else}
              <Clipboard size="18" />
              <span>Paste Session from Clipboard</span>
            {/if}
          </button>
          <p class="text-center text-xs text-gray-500 dark:text-gray-400">
            After clicking "Copy Session & Close" in the login window, click above to paste.
          </p>
        </div>
      {/if}

      <!-- Divider -->
      <div class="relative my-6">
        <div class="absolute inset-0 flex items-center">
          <div class="w-full border-t border-gray-300 dark:border-gray-600"></div>
        </div>
        <div class="relative flex justify-center text-sm">
          <button
            on:click={() => showManualInput = !showManualInput}
            class="px-3 bg-white dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
          >
            {showManualInput ? 'Hide manual setup' : 'Having trouble? Use manual setup'}
          </button>
        </div>
      </div>

      <!-- Manual Cookie Input (Collapsible) -->
      {#if showManualInput}
        <div class="card-secondary p-6 mb-6">
          <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4 flex items-center">
            <Key size="20" class="mr-2 text-primary-600 dark:text-primary-400" />
            Manual Setup
          </h2>
          <ol class="space-y-3 text-gray-600 dark:text-gray-400 text-sm mb-4">
            <li class="flex items-start">
              <span class="flex-shrink-0 w-5 h-5 bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full flex items-center justify-center text-xs font-medium mr-2 mt-0.5">1</span>
              <span>Open <button on:click={openAugmentPortal} class="text-primary-600 dark:text-primary-400 underline hover:no-underline">app.augmentcode.com</button> and log in</span>
            </li>
            <li class="flex items-start">
              <span class="flex-shrink-0 w-5 h-5 bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full flex items-center justify-center text-xs font-medium mr-2 mt-0.5">2</span>
              <span>Open DevTools (Cmd+Option+I) → Application → Cookies</span>
            </li>
            <li class="flex items-start">
              <span class="flex-shrink-0 w-5 h-5 bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full flex items-center justify-center text-xs font-medium mr-2 mt-0.5">3</span>
              <span>Find <code class="bg-gray-200 dark:bg-gray-700 px-1 rounded">_session</code> cookie and copy its value</span>
            </li>
          </ol>

          <form on:submit|preventDefault={handleManualSubmit} class="space-y-4">
            <div class="form-group">
              <label for="session-input" class="form-label flex items-center">
                <Cookie size="16" class="mr-2" />
                Session Cookie (_session)
              </label>
              <textarea
                id="session-input"
                bind:value={sessionCookie}
                placeholder="Paste your _session cookie value here..."
                class="input w-full font-mono text-xs"
                rows="3"
                disabled={isLoading || success}
              ></textarea>
            </div>

            <button
              type="submit"
              class="btn btn-secondary w-full"
              disabled={isLoading || success || !sessionCookie.trim()}
            >
              {#if isLoading}
                <Loader size="16" class="animate-spin" />
                <span>Connecting...</span>
              {:else}
                <span>Connect Manually</span>
              {/if}
            </button>
          </form>
        </div>
      {/if}

      <!-- Security Note -->
      <div class="message-info mt-6">
        <AlertCircle size="16" />
        <div>
          <p class="font-medium">Security & Privacy</p>
          <p class="text-sm opacity-90 mt-1">
            Your session is stored locally and encrypted. We only communicate with Augment's servers to fetch your credit data.
          </p>
        </div>
      </div>
    </div>
  </div>
</div>
