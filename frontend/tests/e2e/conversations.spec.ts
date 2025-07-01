import { test, expect } from '@playwright/test';

test.describe('Conversations Page', () => {
  test.beforeEach(async ({ page }) => {
    // Mock conversations API
    await page.route('/api/conversations*', async route => {
      const url = new URL(route.request().url());
      const page_num = parseInt(url.searchParams.get('page') || '1');
      const per_page = parseInt(url.searchParams.get('per_page') || '20');
      
      const conversations = Array.from({ length: per_page }, (_, i) => ({
        id: `conv-${(page_num - 1) * per_page + i + 1}`,
        title: `Conversation ${(page_num - 1) * per_page + i + 1}`,
        provider: ['chatgpt', 'claude', 'gemini', 'zed'][i % 4],
        start_time: new Date(2023, 0, i + 1).toISOString(),
        end_time: new Date(2023, 0, i + 1, 1).toISOString(),
        message_count: Math.floor(Math.random() * 20) + 1,
      }));
      
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          conversations,
          total: 100,
          page: page_num,
          total_pages: 5,
        }),
      });
    });
    
    await page.goto('/conversations');
  });
  
  test('displays conversation list', async ({ page }) => {
    await expect(page.getByRole('heading', { name: 'Conversations' })).toBeVisible();
    
    // Should show 20 conversations
    const conversations = page.locator('[data-testid="conversation-item"]');
    await expect(conversations).toHaveCount(20);
    
    // Check first conversation
    const firstConv = conversations.first();
    await expect(firstConv).toContainText('Conversation 1');
    await expect(firstConv).toContainText('chatgpt');
  });
  
  test('pagination works correctly', async ({ page }) => {
    // Check pagination controls
    await expect(page.getByText('Page 1 of 5')).toBeVisible();
    
    // Go to next page
    await page.getByRole('button', { name: 'Next' }).click();
    await expect(page).toHaveURL(/page=2/);
    await expect(page.getByText('Conversation 21')).toBeVisible();
    
    // Go to last page
    await page.getByRole('button', { name: 'Last' }).click();
    await expect(page).toHaveURL(/page=5/);
  });
  
  test('filtering by provider works', async ({ page }) => {
    // Select ChatGPT filter
    await page.getByLabel('Provider').selectOption('chatgpt');
    await page.getByRole('button', { name: 'Apply Filters' }).click();
    
    await expect(page).toHaveURL(/provider=chatgpt/);
    
    // All visible conversations should be ChatGPT
    const providers = page.locator('[data-testid="conversation-provider"]');
    const count = await providers.count();
    for (let i = 0; i < count; i++) {
      await expect(providers.nth(i)).toHaveText('chatgpt');
    }
  });
  
  test('date range filtering works', async ({ page }) => {
    await page.getByLabel('Start Date').fill('2023-01-01');
    await page.getByLabel('End Date').fill('2023-01-10');
    await page.getByRole('button', { name: 'Apply Filters' }).click();
    
    await expect(page).toHaveURL(/start_date=2023-01-01/);
    await expect(page).toHaveURL(/end_date=2023-01-10/);
  });
  
  test('clicking conversation navigates to detail page', async ({ page }) => {
    await page.locator('[data-testid="conversation-item"]').first().click();
    
    await expect(page).toHaveURL(/\/conversations\/conv-1/);
  });
  
  test('bulk actions work', async ({ page }) => {
    // Select multiple conversations
    await page.locator('[data-testid="select-conv-1"]').check();
    await page.locator('[data-testid="select-conv-2"]').check();
    await page.locator('[data-testid="select-conv-3"]').check();
    
    // Bulk action menu should appear
    await expect(page.getByText('3 selected')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Export Selected' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Delete Selected' })).toBeVisible();
  });
  
  test('list loads efficiently with many items', async ({ page }) => {
    const startTime = Date.now();
    
    await page.goto('/conversations');
    await page.waitForSelector('[data-testid="conversation-item"]');
    
    const loadTime = Date.now() - startTime;
    expect(loadTime).toBeLessThan(500); // Should load within 500ms
    
    // Check virtual scrolling is working
    const visibleItems = await page.locator('[data-testid="conversation-item"]:visible').count();
    expect(visibleItems).toBeLessThanOrEqual(30); // Should only render visible items
  });
});