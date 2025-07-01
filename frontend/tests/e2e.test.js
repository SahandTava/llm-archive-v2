import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  // Start from search page
  await page.goto('http://localhost:4173');
});

test('search functionality', async ({ page }) => {
  // Focus search input
  await page.keyboard.press('/');
  
  // Type search query
  await page.fill('input[type="search"]', 'rust');
  
  // Measure search performance
  const startTime = Date.now();
  await page.keyboard.press('Enter');
  
  // Wait for results
  await page.waitForSelector('.search-results');
  const endTime = Date.now();
  
  // Should respond in under 500ms
  expect(endTime - startTime).toBeLessThan(500);
  
  // Should show results
  const results = await page.$$('.search-result');
  expect(results.length).toBeGreaterThan(0);
});

test('keyboard navigation', async ({ page }) => {
  // Search for something
  await page.keyboard.press('/');
  await page.fill('input[type="search"]', 'test');
  await page.keyboard.press('Enter');
  
  await page.waitForSelector('.search-results');
  
  // Navigate with j/k
  await page.keyboard.press('j');
  let selected = await page.$('.search-result.selected');
  expect(selected).toBeTruthy();
  
  await page.keyboard.press('k');
  selected = await page.$('.search-result.selected:first-child');
  expect(selected).toBeTruthy();
  
  // Open with Enter
  await page.keyboard.press('Enter');
  await page.waitForURL(/\/conversations\/\d+/);
});

test('conversation view', async ({ page }) => {
  // Navigate to conversations
  await page.goto('http://localhost:4173/conversations');
  
  // Click first conversation
  await page.click('.conversation-item:first-child a');
  
  // Should show messages
  await page.waitForSelector('.message');
  const messages = await page.$$('.message');
  expect(messages.length).toBeGreaterThan(0);
  
  // Check message roles
  const userMessage = await page.$('.message.user');
  const assistantMessage = await page.$('.message.assistant');
  expect(userMessage).toBeTruthy();
  expect(assistantMessage).toBeTruthy();
});

test('export functionality', async ({ page }) => {
  // Navigate to a conversation
  await page.goto('http://localhost:4173/conversations/1');
  
  // Press 'e' to export
  await page.keyboard.press('e');
  
  // Check download started
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.keyboard.press('e')
  ]);
  
  expect(download.suggestedFilename()).toMatch(/conversation.*\.md$/);
});

test('page load performance', async ({ page }) => {
  const routes = [
    '/',
    '/conversations',
    '/conversations/1'
  ];
  
  for (const route of routes) {
    const startTime = Date.now();
    await page.goto(`http://localhost:4173${route}`);
    const endTime = Date.now();
    
    // Page should load in under 500ms
    expect(endTime - startTime).toBeLessThan(500);
  }
});

test('filter conversations', async ({ page }) => {
  await page.goto('http://localhost:4173/conversations');
  
  // Filter by provider
  await page.selectOption('select[name="provider"]', 'chatgpt');
  
  // Check URL updated
  expect(page.url()).toContain('provider=chatgpt');
  
  // Check results filtered
  const items = await page.$$('.conversation-item');
  for (const item of items) {
    const provider = await item.$eval('.provider', el => el.textContent);
    expect(provider).toBe('ChatGPT');
  }
});

test('search highlighting', async ({ page }) => {
  await page.keyboard.press('/');
  await page.fill('input[type="search"]', 'specific term');
  await page.keyboard.press('Enter');
  
  await page.waitForSelector('.search-results');
  
  // Check that search term is highlighted
  const highlighted = await page.$$('mark');
  expect(highlighted.length).toBeGreaterThan(0);
});

test('responsive behavior', async ({ page }) => {
  // Desktop fixed width
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto('http://localhost:4173');
  
  const container = await page.$('.container');
  const box = await container.boundingBox();
  expect(box.width).toBe(1200);
});

test('no loading spinners', async ({ page }) => {
  await page.goto('http://localhost:4173');
  
  // Should not have any loading indicators
  const spinners = await page.$$('.spinner, .loading, .loader');
  expect(spinners.length).toBe(0);
  
  // Content should be immediately visible
  const searchInput = await page.$('input[type="search"]');
  expect(await searchInput.isVisible()).toBe(true);
});