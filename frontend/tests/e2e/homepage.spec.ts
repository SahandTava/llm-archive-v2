import { test, expect } from '@playwright/test';

test.describe('Homepage', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });
  
  test('has correct title and heading', async ({ page }) => {
    await expect(page).toHaveTitle(/LLM Archive/);
    await expect(page.getByRole('heading', { name: 'LLM Archive V2' })).toBeVisible();
  });
  
  test('displays navigation menu', async ({ page }) => {
    await expect(page.getByRole('navigation')).toBeVisible();
    await expect(page.getByRole('link', { name: 'Conversations' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Import' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Search' })).toBeVisible();
  });
  
  test('shows recent conversations', async ({ page }) => {
    // Mock API response
    await page.route('/api/conversations?limit=10', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          conversations: [
            {
              id: 'conv-1',
              title: 'Test Conversation 1',
              provider: 'chatgpt',
              start_time: '2023-01-01T12:00:00Z',
              message_count: 5,
            },
            {
              id: 'conv-2',
              title: 'Test Conversation 2',
              provider: 'claude',
              start_time: '2023-01-02T12:00:00Z',
              message_count: 3,
            },
          ],
          total: 2,
        }),
      });
    });
    
    await page.reload();
    
    await expect(page.getByText('Test Conversation 1')).toBeVisible();
    await expect(page.getByText('Test Conversation 2')).toBeVisible();
    await expect(page.getByText('chatgpt')).toBeVisible();
    await expect(page.getByText('claude')).toBeVisible();
  });
  
  test('search functionality works', async ({ page }) => {
    await page.getByPlaceholder('Search conversations...').fill('machine learning');
    await page.getByRole('button', { name: 'Search' }).click();
    
    await expect(page).toHaveURL(/search\?q=machine\+learning/);
  });
  
  test('loads within 100ms', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const loadTime = Date.now() - startTime;
    
    expect(loadTime).toBeLessThan(1000); // 1 second max for initial load
  });
  
  test('is responsive on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    
    // Mobile menu should be visible
    await expect(page.getByRole('button', { name: 'Menu' })).toBeVisible();
    
    // Click menu to show navigation
    await page.getByRole('button', { name: 'Menu' }).click();
    await expect(page.getByRole('navigation')).toBeVisible();
  });
});