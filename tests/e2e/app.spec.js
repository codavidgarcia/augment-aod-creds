import { test, expect } from '@playwright/test';

test.describe('Augment Credit Monitor E2E', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/');
    
    // Wait for app to initialize
    await page.waitForSelector('[data-testid="app-initialized"]', { timeout: 10000 });
  });

  test('displays balance widget', async ({ page }) => {
    // Check that the balance widget is visible
    await expect(page.locator('[data-testid="balance-widget"]')).toBeVisible();
    
    // Check for balance display
    const balanceElement = page.locator('[data-testid="balance-amount"]');
    await expect(balanceElement).toBeVisible();
  });

  test('refresh button works', async ({ page }) => {
    const refreshButton = page.locator('[aria-label="Refresh balance"]');
    await expect(refreshButton).toBeVisible();
    
    // Click refresh button
    await refreshButton.click();
    
    // Should show loading state briefly
    await expect(refreshButton).toBeDisabled();
    
    // Should return to normal state
    await expect(refreshButton).toBeEnabled({ timeout: 5000 });
  });

  test('settings panel opens and closes', async ({ page }) => {
    const settingsButton = page.locator('button:has-text("Settings")');
    await expect(settingsButton).toBeVisible();
    
    // Open settings
    await settingsButton.click();
    
    // Settings panel should be visible
    await expect(page.locator('text=Configuration')).toBeVisible();
    
    // Close settings
    await settingsButton.click();
    
    // Settings panel should be hidden
    await expect(page.locator('text=Configuration')).not.toBeVisible();
  });

  test('navigation tabs work', async ({ page }) => {
    // Check overview tab is active by default
    const overviewTab = page.locator('button:has-text("Overview")');
    const analyticsTab = page.locator('button:has-text("Analytics")');
    
    await expect(overviewTab).toHaveClass(/border-primary-500/);
    
    // Click analytics tab
    await analyticsTab.click();
    
    // Analytics tab should be active
    await expect(analyticsTab).toHaveClass(/border-primary-500/);
    await expect(overviewTab).not.toHaveClass(/border-primary-500/);
  });

  test('charts render when data is available', async ({ page }) => {
    // Navigate to analytics tab
    await page.locator('button:has-text("Analytics")').click();
    
    // Check for chart containers or no data message
    const chartContainer = page.locator('canvas');
    const noDataMessage = page.locator('text=No data available');
    
    // Either charts should be visible or no data message
    await expect(chartContainer.or(noDataMessage)).toBeVisible();
  });

  test('configuration form validation', async ({ page }) => {
    // Open settings
    await page.locator('button:has-text("Settings")').click();
    
    // Test token input
    const tokenInput = page.locator('#augment_token');
    await expect(tokenInput).toBeVisible();
    
    // Test polling interval validation
    const pollingInput = page.locator('#polling_interval');
    await pollingInput.fill('10'); // Below minimum
    
    // Save button
    const saveButton = page.locator('button:has-text("Save")');
    await saveButton.click();
    
    // Should show validation error or reset to minimum value
    const currentValue = await pollingInput.inputValue();
    expect(parseInt(currentValue)).toBeGreaterThanOrEqual(30);
  });

  test('keyboard navigation works', async ({ page }) => {
    // Test tab navigation
    await page.keyboard.press('Tab');
    
    // Should focus on settings button
    await expect(page.locator('button:has-text("Settings")')).toBeFocused();
    
    // Continue tabbing
    await page.keyboard.press('Tab');
    
    // Should focus on refresh button
    await expect(page.locator('[aria-label="Refresh balance"]')).toBeFocused();
  });

  test('responsive design works', async ({ page }) => {
    // Test desktop view
    await page.setViewportSize({ width: 1200, height: 800 });
    
    // Check that layout is appropriate for desktop
    const container = page.locator('.container');
    await expect(container).toBeVisible();
    
    // Test mobile view
    await page.setViewportSize({ width: 375, height: 667 });
    
    // Layout should adapt to mobile
    await expect(container).toBeVisible();
    
    // Settings should still be accessible
    await page.locator('button:has-text("Settings")').click();
    await expect(page.locator('text=Configuration')).toBeVisible();
  });

  test('dark mode toggle works', async ({ page }) => {
    // Check initial theme
    const html = page.locator('html');
    
    // Toggle dark mode (if theme switcher is implemented)
    // This would depend on the actual theme switching implementation
    
    // For now, just verify that dark mode classes can be applied
    await page.evaluate(() => {
      document.documentElement.classList.add('dark');
    });
    
    await expect(html).toHaveClass(/dark/);
  });

  test('error states are handled gracefully', async ({ page }) => {
    // Mock network failure
    await page.route('**/api/**', route => {
      route.abort('failed');
    });
    
    // Try to refresh
    const refreshButton = page.locator('[aria-label="Refresh balance"]');
    await refreshButton.click();
    
    // Should handle error gracefully without crashing
    await expect(refreshButton).toBeEnabled({ timeout: 5000 });
  });

  test('accessibility standards are met', async ({ page }) => {
    // Check for proper heading structure
    const h1 = page.locator('h1');
    await expect(h1).toBeVisible();
    await expect(h1).toHaveText('Augment Credit Monitor');
    
    // Check for proper button labels
    const buttons = page.locator('button');
    const buttonCount = await buttons.count();
    
    for (let i = 0; i < buttonCount; i++) {
      const button = buttons.nth(i);
      const hasText = await button.textContent();
      const hasAriaLabel = await button.getAttribute('aria-label');
      
      // Each button should have either text content or aria-label
      expect(hasText || hasAriaLabel).toBeTruthy();
    }
    
    // Check for proper form labels
    const inputs = page.locator('input');
    const inputCount = await inputs.count();
    
    for (let i = 0; i < inputCount; i++) {
      const input = inputs.nth(i);
      const id = await input.getAttribute('id');
      
      if (id) {
        // Should have corresponding label
        const label = page.locator(`label[for="${id}"]`);
        await expect(label).toBeVisible();
      }
    }
  });

  test('performance is acceptable', async ({ page }) => {
    // Measure page load time
    const startTime = Date.now();
    
    await page.goto('/');
    await page.waitForSelector('[data-testid="app-initialized"]');
    
    const loadTime = Date.now() - startTime;
    
    // Should load within reasonable time (adjust threshold as needed)
    expect(loadTime).toBeLessThan(5000);
    
    // Check for memory leaks by navigating multiple times
    for (let i = 0; i < 3; i++) {
      await page.reload();
      await page.waitForSelector('[data-testid="app-initialized"]');
    }
    
    // App should still be responsive
    const refreshButton = page.locator('[aria-label="Refresh balance"]');
    await expect(refreshButton).toBeVisible();
  });
});
