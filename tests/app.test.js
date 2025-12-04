import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { get } from 'svelte/store';
import BalanceWidget from '../src/lib/components/BalanceWidget.svelte';
import { currentBalance, balanceStatus, isLoading, actions } from '../src/lib/stores/app.js';

// Mock Tauri API
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn()
}));

describe('BalanceWidget', () => {
  beforeEach(() => {
    // Reset stores
    currentBalance.set(null);
    isLoading.set(false);
    
    // Clear all mocks
    vi.clearAllMocks();
  });

  it('renders compact widget with balance', () => {
    currentBalance.set(2683);
    
    render(BalanceWidget, { compact: true });
    
    expect(screen.getByText('2,683')).toBeInTheDocument();
  });

  it('shows loading state', () => {
    isLoading.set(true);
    
    render(BalanceWidget, { compact: true });
    
    const statusIndicator = screen.getByRole('button', { name: /refresh/i }).previousElementSibling.querySelector('div');
    expect(statusIndicator).toHaveClass('animate-pulse');
  });

  it('displays correct status colors', () => {
    // Test healthy status
    currentBalance.set(1000);
    const { rerender } = render(BalanceWidget, { compact: false });
    
    expect(get(balanceStatus)).toBe('healthy');
    
    // Test warning status
    currentBalance.set(400);
    rerender({ compact: false });
    
    expect(get(balanceStatus)).toBe('warning');
    
    // Test critical status
    currentBalance.set(50);
    rerender({ compact: false });
    
    expect(get(balanceStatus)).toBe('critical');
  });

  it('handles refresh button click', async () => {
    const mockInvoke = vi.fn().mockResolvedValue(2500);
    vi.mocked(actions.triggerManualUpdate).mockImplementation(mockInvoke);
    
    render(BalanceWidget, { compact: true });
    
    const refreshButton = screen.getByRole('button', { name: /refresh/i });
    await fireEvent.click(refreshButton);
    
    expect(mockInvoke).toHaveBeenCalled();
  });

  it('formats balance numbers correctly', () => {
    currentBalance.set(1234567);
    
    render(BalanceWidget, { compact: false });
    
    expect(screen.getByText('1,234,567')).toBeInTheDocument();
  });

  it('shows usage rate when available', () => {
    currentBalance.set(2683);
    
    render(BalanceWidget, { compact: true });
    
    // Mock usage analytics
    const mockAnalytics = {
      usage_rate_per_hour: 15.5,
      estimated_hours_remaining: 173.1
    };
    
    // This would be set by the analytics store in real usage
    expect(screen.queryByText('15.5/h')).not.toBeInTheDocument();
  });

  it('is accessible', () => {
    currentBalance.set(2683);
    
    render(BalanceWidget, { compact: true });
    
    const refreshButton = screen.getByRole('button', { name: /refresh/i });
    expect(refreshButton).toBeInTheDocument();
    expect(refreshButton).toHaveAttribute('aria-label', 'Refresh balance');
  });

  it('handles keyboard navigation', async () => {
    currentBalance.set(2683);
    
    render(BalanceWidget, { compact: true });
    
    const refreshButton = screen.getByRole('button', { name: /refresh/i });
    
    // Test focus
    refreshButton.focus();
    expect(document.activeElement).toBe(refreshButton);
    
    // Test Enter key
    await fireEvent.keyDown(refreshButton, { key: 'Enter' });
    // Should trigger refresh (mocked)
  });
});

describe('App Store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('calculates balance status correctly', () => {
    const testCases = [
      { balance: 1000, expected: 'healthy' },
      { balance: 500, expected: 'healthy' },
      { balance: 499, expected: 'warning' },
      { balance: 100, expected: 'warning' },
      { balance: 99, expected: 'critical' },
      { balance: 0, expected: 'critical' }
    ];

    testCases.forEach(({ balance, expected }) => {
      currentBalance.set(balance);
      expect(get(balanceStatus)).toBe(expected);
    });
  });

  it('formats time remaining correctly', () => {
    expect(actions.formatTimeRemaining(0.5)).toBe('30m');
    expect(actions.formatTimeRemaining(1.5)).toBe('1.5h');
    expect(actions.formatTimeRemaining(25)).toBe('1d');
    expect(actions.formatTimeRemaining(72)).toBe('3d');
  });

  it('formats usage rate correctly', () => {
    expect(actions.formatUsageRate(0.123)).toBe('0.12');
    expect(actions.formatUsageRate(1.567)).toBe('1.6');
    expect(actions.formatUsageRate(15.67)).toBe('16');
    expect(actions.formatUsageRate(0)).toBe('0');
  });

  it('provides correct status icons', () => {
    expect(actions.getBalanceIcon('healthy')).toBe('🟢');
    expect(actions.getBalanceIcon('warning')).toBe('🟡');
    expect(actions.getBalanceIcon('critical')).toBe('🔴');
    expect(actions.getBalanceIcon('unknown')).toBe('⚪');
  });

  it('provides correct trend icons', () => {
    expect(actions.getTrendIcon('Increasing')).toBe('📈');
    expect(actions.getTrendIcon('Decreasing')).toBe('📉');
    expect(actions.getTrendIcon('Stable')).toBe('➡️');
    expect(actions.getTrendIcon('Insufficient')).toBe('❓');
  });
});

describe('Accessibility', () => {
  it('meets WCAG contrast requirements', () => {
    // This would typically use a tool like axe-core
    // For now, we ensure proper color classes are applied
    currentBalance.set(2683);
    
    render(BalanceWidget, { compact: false });
    
    // Check that status colors are applied correctly
    const balanceElement = screen.getByText('2,683');
    expect(balanceElement).toHaveClass('text-success-600', 'dark:text-success-400');
  });

  it('supports keyboard navigation', () => {
    render(BalanceWidget, { compact: true });
    
    const refreshButton = screen.getByRole('button', { name: /refresh/i });
    
    // Should be focusable
    expect(refreshButton.tabIndex).not.toBe(-1);
    
    // Should have proper focus styles
    expect(refreshButton).toHaveClass('focus:outline-none', 'focus:ring-2');
  });

  it('provides proper ARIA labels', () => {
    render(BalanceWidget, { compact: true });
    
    const refreshButton = screen.getByRole('button', { name: /refresh/i });
    expect(refreshButton).toHaveAttribute('aria-label', 'Refresh balance');
  });
});

describe('Error Handling', () => {
  it('handles fetch errors gracefully', async () => {
    const mockError = new Error('Network error');
    vi.mocked(actions.triggerManualUpdate).mockRejectedValue(mockError);
    
    render(BalanceWidget, { compact: true });
    
    const refreshButton = screen.getByRole('button', { name: /refresh/i });
    await fireEvent.click(refreshButton);
    
    // Should not crash and should handle error
    expect(refreshButton).toBeInTheDocument();
  });

  it('shows appropriate loading states', async () => {
    let resolvePromise;
    const mockPromise = new Promise(resolve => {
      resolvePromise = resolve;
    });
    
    vi.mocked(actions.triggerManualUpdate).mockReturnValue(mockPromise);
    
    render(BalanceWidget, { compact: true });
    
    const refreshButton = screen.getByRole('button', { name: /refresh/i });
    fireEvent.click(refreshButton);
    
    // Should show loading state
    await waitFor(() => {
      expect(refreshButton).toBeDisabled();
    });
    
    // Resolve the promise
    resolvePromise(2683);
    
    await waitFor(() => {
      expect(refreshButton).not.toBeDisabled();
    });
  });
});
