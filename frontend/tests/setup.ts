// Test setup for Vitest
import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock SvelteKit modules
vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
  invalidate: vi.fn(),
  invalidateAll: vi.fn(),
  preloadData: vi.fn(),
  preloadCode: vi.fn(),
  pushState: vi.fn(),
  replaceState: vi.fn(),
}));

vi.mock('$app/stores', () => ({
  page: {
    subscribe: vi.fn(),
  },
  navigating: {
    subscribe: vi.fn(),
  },
  updated: {
    subscribe: vi.fn(),
    check: vi.fn(),
  },
}));

vi.mock('$app/environment', () => ({
  browser: true,
  dev: true,
  building: false,
  version: 'test',
}));

// Mock fetch for API calls
global.fetch = vi.fn();

// Setup test utilities
export const mockFetch = (data: any, options: { status?: number; ok?: boolean } = {}) => {
  const { status = 200, ok = true } = options;
  
  return vi.mocked(fetch).mockResolvedValueOnce({
    ok,
    status,
    json: () => Promise.resolve(data),
    text: () => Promise.resolve(JSON.stringify(data)),
    headers: new Headers({
      'content-type': 'application/json',
    }),
  } as Response);
};

// Performance timer for tests
export class TestTimer {
  private start: number;
  
  constructor(private name: string) {
    this.start = performance.now();
  }
  
  assertUnderMs(maxMs: number) {
    const elapsed = performance.now() - this.start;
    if (elapsed > maxMs) {
      throw new Error(`${this.name} took ${elapsed.toFixed(2)}ms, expected < ${maxMs}ms`);
    }
  }
}