import { test, expect } from '@playwright/test';
import path from 'path';

test.describe('Import Functionality', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/import');
  });
  
  test('displays import page correctly', async ({ page }) => {
    await expect(page.getByRole('heading', { name: 'Import Conversations' })).toBeVisible();
    
    // Provider selection
    await expect(page.getByLabel('Select Provider')).toBeVisible();
    await expect(page.getByRole('option', { name: 'ChatGPT' })).toBeInViewport();
    await expect(page.getByRole('option', { name: 'Claude' })).toBeInViewport();
    await expect(page.getByRole('option', { name: 'Gemini' })).toBeInViewport();
    await expect(page.getByRole('option', { name: 'Zed' })).toBeInViewport();
    
    // File upload area
    await expect(page.getByText('Drop files here or click to browse')).toBeVisible();
  });
  
  test('imports ChatGPT conversations', async ({ page }) => {
    // Select ChatGPT
    await page.getByLabel('Select Provider').selectOption('chatgpt');
    
    // Upload test file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(__dirname, '../fixtures/chatgpt-export.json'));
    
    // Mock import API
    await page.route('/api/import', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          imported_count: 5,
          skipped_count: 0,
          errors: [],
        }),
      });
    });
    
    // Click import button
    await page.getByRole('button', { name: 'Import' }).click();
    
    // Check success message
    await expect(page.getByText('Successfully imported 5 conversations')).toBeVisible();
  });
  
  test('handles multiple file upload', async ({ page }) => {
    await page.getByLabel('Select Provider').selectOption('claude');
    
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles([
      path.join(__dirname, '../fixtures/claude-export-1.json'),
      path.join(__dirname, '../fixtures/claude-export-2.json'),
    ]);
    
    // Should show file list
    await expect(page.getByText('claude-export-1.json')).toBeVisible();
    await expect(page.getByText('claude-export-2.json')).toBeVisible();
    
    // Should show remove buttons
    await expect(page.getByRole('button', { name: 'Remove' })).toHaveCount(2);
  });
  
  test('shows import progress', async ({ page }) => {
    await page.getByLabel('Select Provider').selectOption('gemini');
    
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(__dirname, '../fixtures/large-export.json'));
    
    // Mock slow import
    await page.route('/api/import', async route => {
      await new Promise(resolve => setTimeout(resolve, 2000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          imported_count: 100,
          skipped_count: 5,
          errors: [],
        }),
      });
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    // Should show progress indicator
    await expect(page.getByRole('progressbar')).toBeVisible();
    await expect(page.getByText('Importing...')).toBeVisible();
    
    // Wait for completion
    await expect(page.getByText('Successfully imported 100 conversations')).toBeVisible({ timeout: 5000 });
  });
  
  test('handles import errors gracefully', async ({ page }) => {
    await page.getByLabel('Select Provider').selectOption('chatgpt');
    
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(__dirname, '../fixtures/invalid-export.json'));
    
    // Mock error response
    await page.route('/api/import', async route => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'Invalid file format',
          details: 'Expected JSON array but got object',
        }),
      });
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    // Should show error message
    await expect(page.getByText('Import failed: Invalid file format')).toBeVisible();
    await expect(page.getByText('Expected JSON array but got object')).toBeVisible();
  });
  
  test('drag and drop works', async ({ page }) => {
    await page.getByLabel('Select Provider').selectOption('claude');
    
    // Create a data transfer for drag and drop
    const dataTransfer = await page.evaluateHandle(() => new DataTransfer());
    
    // Simulate drag over
    const dropZone = page.locator('[data-testid="drop-zone"]');
    await dropZone.dispatchEvent('dragover', { dataTransfer });
    
    // Drop zone should highlight
    await expect(dropZone).toHaveClass(/drag-over/);
    
    // Note: Full drag-and-drop file testing requires more complex setup
  });
  
  test('performance: handles large files efficiently', async ({ page }) => {
    await page.getByLabel('Select Provider').selectOption('chatgpt');
    
    // Note: In real tests, create a large test file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(__dirname, '../fixtures/large-export.json'));
    
    const startTime = Date.now();
    
    // File should be processed quickly client-side
    await expect(page.getByText('large-export.json')).toBeVisible();
    
    const processingTime = Date.now() - startTime;
    expect(processingTime).toBeLessThan(100); // Should process file info within 100ms
  });
});