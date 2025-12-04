import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock Tauri API
global.__TAURI__ = {
  invoke: vi.fn(),
  event: {
    listen: vi.fn(),
    emit: vi.fn(),
  },
  window: {
    appWindow: {
      hide: vi.fn(),
      show: vi.fn(),
      minimize: vi.fn(),
      close: vi.fn(),
    },
  },
  notification: {
    sendNotification: vi.fn(),
  },
};

// Mock Chart.js
vi.mock('chart.js/auto', () => ({
  default: vi.fn(() => ({
    destroy: vi.fn(),
    update: vi.fn(),
    data: { datasets: [{ data: [] }] },
  })),
}));

// Mock chartjs-adapter-date-fns
vi.mock('chartjs-adapter-date-fns', () => ({}));

// Mock Lucide Svelte icons
vi.mock('lucide-svelte', () => ({
  RefreshCw: 'div',
  Settings: 'div',
  TrendingDown: 'div',
  TrendingUp: 'div',
  Clock: 'div',
  Zap: 'div',
  Bell: 'div',
  BarChart3: 'div',
}));

// Setup DOM environment
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // deprecated
    removeListener: vi.fn(), // deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock IntersectionObserver
global.IntersectionObserver = vi.fn(() => ({
  observe: vi.fn(),
  disconnect: vi.fn(),
  unobserve: vi.fn(),
}));

// Mock ResizeObserver
global.ResizeObserver = vi.fn(() => ({
  observe: vi.fn(),
  disconnect: vi.fn(),
  unobserve: vi.fn(),
}));

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};
global.localStorage = localStorageMock;

// Mock fetch
global.fetch = vi.fn();

// Console error suppression for expected errors in tests
const originalError = console.error;
console.error = (...args) => {
  if (
    typeof args[0] === 'string' &&
    args[0].includes('Warning: ReactDOM.render is no longer supported')
  ) {
    return;
  }
  originalError.call(console, ...args);
};
