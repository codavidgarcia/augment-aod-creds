<script>
  import { onMount } from 'svelte';
  import { initializeApp } from '../lib/stores/app.js';
  import '../app.css';

  let appInitialized = false;
  let initError = null;

  onMount(async () => {
    try {
      console.log('🚀 Layout: Starting app initialization...');

      // Add timeout to prevent hanging, but be more generous
      const initPromise = initializeApp();
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Initialization timeout after 15 seconds')), 15000)
      );

      const success = await Promise.race([initPromise, timeoutPromise]);

      console.log('✅ Layout: App initialization completed, success:', success);
      appInitialized = true; // Always allow UI to load

    } catch (error) {
      console.error('❌ Layout: App initialization error:', error);
      console.log('⚠️ Layout: Allowing UI to load despite initialization error');

      // Still allow the UI to load, just show a warning
      appInitialized = true;
      initError = `Initialization warning: ${error.message}. The app may have limited functionality.`;

      // Clear the error after a few seconds so it doesn't stay forever
      setTimeout(() => {
        initError = null;
      }, 10000);
    }
  });
</script>

<main class="min-h-screen bg-gray-50 dark:bg-gray-900">
  {#if !appInitialized && !initError}
    <!-- Loading State -->
    <div class="flex items-center justify-center min-h-screen">
      <div class="text-center space-y-4">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto"></div>
        <div class="text-sm text-gray-600 dark:text-gray-400">
          Initializing Augment Credit Monitor...
        </div>
      </div>
    </div>
  {:else if initError}
    <!-- Error State -->
    <div class="flex items-center justify-center min-h-screen">
      <div class="text-center space-y-4 max-w-md mx-auto p-6">
        <div class="text-4xl">⚠️</div>
        <h1 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Initialization Failed
        </h1>
        <p class="text-sm text-gray-600 dark:text-gray-400">
          {initError}
        </p>
        <button 
          class="btn btn-primary"
          on:click={() => window.location.reload()}
        >
          Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- App Content -->
    <slot />
  {/if}
</main>
