import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import { join } from 'path';

test.describe('Import Scenarios E2E', () => {
  test.beforeEach(async ({ page }) => {
    // Start with clean state
    await page.goto('/');
    
    // Mock backend import endpoint
    await page.route('/api/import', async route => {
      const request = route.request();
      const formData = await request.postDataBuffer();
      
      // Simulate processing time based on file size
      const fileSize = formData?.length || 0;
      const processingTime = Math.min(fileSize / 1000, 2000); // Max 2s
      
      await new Promise(resolve => setTimeout(resolve, processingTime));
      
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          imported_count: Math.floor(Math.random() * 10) + 1,
          errors: [],
          processing_time_ms: processingTime,
          duplicates_found: Math.floor(Math.random() * 3)
        })
      });
    });
    
    // Navigate to import page
    await page.goto('/import');
  });

  test('should handle ChatGPT export import flow', async ({ page }) => {
    await expect(page.getByRole('heading', { name: 'Import Conversations' })).toBeVisible();
    
    // Select provider
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    // Upload file
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'chatgpt-export.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify([{
        title: 'Test ChatGPT Import',
        create_time: Date.now() / 1000,
        mapping: {
          'root': {
            children: ['msg1']
          },
          'msg1': {
            message: {
              author: { role: 'user' },
              content: { parts: ['Hello'] },
              create_time: Date.now() / 1000
            },
            children: ['msg2']
          },
          'msg2': {
            message: {
              author: { role: 'assistant' },
              content: { parts: ['Hi there!'] },
              create_time: Date.now() / 1000
            },
            children: []
          }
        }
      }]))
    });
    
    // Start import
    await page.getByRole('button', { name: 'Import' }).click();
    
    // Should show processing indicator
    await expect(page.getByText('Processing import...')).toBeVisible();
    
    // Should show progress bar
    await expect(page.locator('[data-testid="import-progress"]')).toBeVisible();
    
    // Wait for completion
    await expect(page.getByText('Import completed successfully!')).toBeVisible({ timeout: 5000 });
    
    // Should display results
    await expect(page.getByText(/Imported \d+ conversations/)).toBeVisible();
    await expect(page.getByText(/Found \d+ duplicates/)).toBeVisible();
    
    // Should offer to view imported conversations
    await expect(page.getByRole('link', { name: 'View Imported Conversations' })).toBeVisible();
  });

  test('should handle Claude export import flow', async ({ page }) => {
    await page.getByLabel('Provider').selectOption('claude');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'claude-export.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify([{
        uuid: 'test-uuid-123',
        name: 'Test Claude Import',
        created_at: new Date().toISOString(),
        chat_messages: [
          {
            uuid: 'msg1',
            text: 'Hello Claude',
            sender: 'human',
            created_at: new Date().toISOString()
          },
          {
            uuid: 'msg2', 
            text: 'Hello! How can I help?',
            sender: 'assistant',
            created_at: new Date().toISOString()
          }
        ]
      }]))
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    // Claude imports should be fast
    await expect(page.getByText('Import completed successfully!')).toBeVisible({ timeout: 3000 });
  });

  test('should handle Gemini Takeout import flow', async ({ page }) => {
    await page.getByLabel('Provider').selectOption('gemini');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'Takeout-Bard.json', 
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify([{
        conversation_id: 'conv-123',
        turns: [
          {
            user_input: {
              text: 'Hello Gemini'
            },
            timestamp: new Date().toISOString()
          },
          {
            model_output: {
              text: 'Hello! I\'m Gemini, how can I help?'
            },
            timestamp: new Date().toISOString()
          }
        ]
      }]))
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    await expect(page.getByText('Import completed successfully!')).toBeVisible({ timeout: 3000 });
  });

  test('should handle Zed import flow', async ({ page }) => {
    await page.getByLabel('Provider').selectOption('zed');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'conversation.json',
      mimeType: 'application/json', 
      buffer: Buffer.from(JSON.stringify({
        id: 'zed-conv-123',
        messages: [
          {
            role: 'user',
            content: 'Help me debug this code',
            timestamp: new Date().toISOString()
          },
          {
            role: 'assistant',
            content: 'I can help with debugging. What\'s the issue?',
            timestamp: new Date().toISOString()
          }
        ]
      }))
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    await expect(page.getByText('Import completed successfully!')).toBeVisible({ timeout: 3000 });
  });

  test('should handle large file import with progress tracking', async ({ page }) => {
    // Create large test file (simulate 100 conversations)
    const largeData = Array.from({ length: 100 }, (_, i) => ({
      title: `Large Import Conversation ${i}`,
      create_time: Date.now() / 1000,
      mapping: {
        'root': { children: [`msg1-${i}`] },
        [`msg1-${i}`]: {
          message: {
            author: { role: 'user' },
            content: { parts: [`Message ${i}`] },
            create_time: Date.now() / 1000
          },
          children: [`msg2-${i}`]
        },
        [`msg2-${i}`]: {
          message: {
            author: { role: 'assistant' },
            content: { parts: [`Response ${i}`] },
            create_time: Date.now() / 1000
          },
          children: []
        }
      }
    }));
    
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'large-export.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify(largeData))
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    // Should show processing state immediately
    await expect(page.getByText('Processing import...')).toBeVisible();
    
    // Progress bar should be visible and updating
    const progressBar = page.locator('[data-testid="import-progress"]');
    await expect(progressBar).toBeVisible();
    
    // Should show file size info
    await expect(page.getByText(/Processing.*KB/)).toBeVisible();
    
    // Should complete within reasonable time (under 5 seconds for large file)
    await expect(page.getByText('Import completed successfully!')).toBeVisible({ timeout: 5000 });
    
    // Should show meaningful results
    await expect(page.getByText(/Imported \d+ conversations/)).toBeVisible();
    await expect(page.getByText(/Processing time:/)).toBeVisible();
  });

  test('should handle import errors gracefully', async ({ page }) => {
    // Mock error response
    await page.route('/api/import', async route => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'Invalid file format',
          details: 'The uploaded file does not match the expected ChatGPT export format'
        })
      });
    });
    
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'invalid.json',
      mimeType: 'application/json',
      buffer: Buffer.from('invalid json content')
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    // Should show error message
    await expect(page.getByText('Import failed')).toBeVisible();
    await expect(page.getByText('Invalid file format')).toBeVisible();
    await expect(page.getByText('The uploaded file does not match')).toBeVisible();
    
    // Should offer retry option
    await expect(page.getByRole('button', { name: 'Try Again' })).toBeVisible();
  });

  test('should validate file before upload', async ({ page }) => {
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    // Try to upload non-JSON file
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'document.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('This is not a JSON file')
    });
    
    // Should show validation error
    await expect(page.getByText('Please select a JSON file')).toBeVisible();
    
    // Import button should be disabled
    await expect(page.getByRole('button', { name: 'Import' })).toBeDisabled();
  });

  test('should show file size warning for large files', async ({ page }) => {
    // Create very large file (simulate 10MB)
    const veryLargeData = Array.from({ length: 1000 }, (_, i) => ({
      title: `Very Large Import ${i}`,
      content: 'A'.repeat(10000) // Large content per item
    }));
    
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'very-large-export.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify(veryLargeData))
    });
    
    // Should show size warning
    await expect(page.getByText(/Large file detected.*may take longer/)).toBeVisible();
    
    // Should still allow import
    await expect(page.getByRole('button', { name: 'Import' })).toBeEnabled();
  });

  test('should handle duplicate detection during import', async ({ page }) => {
    // Mock response with duplicates
    await page.route('/api/import', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          imported_count: 5,
          errors: [],
          duplicates_found: 3,
          duplicate_details: [
            { title: 'Duplicate Conversation 1', reason: 'Same title and timestamp' },
            { title: 'Duplicate Conversation 2', reason: 'Same content hash' },
            { title: 'Duplicate Conversation 3', reason: 'Same provider conversation ID' }
          ]
        })
      });
    });
    
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'with-duplicates.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify([/* some data */]))
    });
    
    await page.getByRole('button', { name: 'Import' }).click();
    
    await expect(page.getByText('Import completed successfully!')).toBeVisible();
    
    // Should show duplicate information
    await expect(page.getByText('Found 3 duplicates')).toBeVisible();
    await expect(page.getByText('Imported 5 new conversations')).toBeVisible();
    
    // Should offer to view duplicate details
    await page.getByRole('button', { name: 'View Duplicate Details' }).click();
    
    await expect(page.getByText('Duplicate Conversation 1')).toBeVisible();
    await expect(page.getByText('Same title and timestamp')).toBeVisible();
  });

  test('should support batch import of multiple files', async ({ page }) => {
    // Enable batch import mode
    await page.getByLabel('Batch Import Mode').check();
    
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    // Upload multiple files
    const fileInput = page.getByLabel('Choose files');
    await fileInput.setInputFiles([
      {
        name: 'export1.json',
        mimeType: 'application/json',
        buffer: Buffer.from(JSON.stringify([{ title: 'File 1 Conversation' }]))
      },
      {
        name: 'export2.json', 
        mimeType: 'application/json',
        buffer: Buffer.from(JSON.stringify([{ title: 'File 2 Conversation' }]))
      },
      {
        name: 'export3.json',
        mimeType: 'application/json', 
        buffer: Buffer.from(JSON.stringify([{ title: 'File 3 Conversation' }]))
      }
    ]);
    
    // Should show file list
    await expect(page.getByText('3 files selected')).toBeVisible();
    await expect(page.getByText('export1.json')).toBeVisible();
    await expect(page.getByText('export2.json')).toBeVisible();
    await expect(page.getByText('export3.json')).toBeVisible();
    
    await page.getByRole('button', { name: 'Import All' }).click();
    
    // Should process files sequentially
    await expect(page.getByText('Processing file 1 of 3...')).toBeVisible();
    
    // Should show overall progress
    await expect(page.locator('[data-testid="batch-progress"]')).toBeVisible();
    
    await expect(page.getByText('Batch import completed successfully!')).toBeVisible({ timeout: 10000 });
    
    // Should show combined results
    await expect(page.getByText(/Total imported: \d+ conversations/)).toBeVisible();
  });

  test('should maintain performance standards during import', async ({ page }) => {
    const performanceData: number[] = [];
    
    // Monitor performance during import
    page.on('response', response => {
      if (response.url().includes('/api/import')) {
        const timing = response.timing();
        performanceData.push(timing.responseEnd - timing.requestStart);
      }
    });
    
    await page.getByLabel('Provider').selectOption('chatgpt');
    
    const fileInput = page.getByLabel('Choose file');
    await fileInput.setInputFiles({
      name: 'performance-test.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify(Array.from({ length: 50 }, (_, i) => ({
        title: `Performance Test ${i}`,
        messages: Array.from({ length: 10 }, (_, j) => ({
          role: j % 2 === 0 ? 'user' : 'assistant',
          content: `Message ${j} in conversation ${i}`
        }))
      }))))
    });
    
    const startTime = Date.now();
    
    await page.getByRole('button', { name: 'Import' }).click();
    await expect(page.getByText('Import completed successfully!')).toBeVisible({ timeout: 5000 });
    
    const endTime = Date.now();
    const totalTime = endTime - startTime;
    
    // Import should complete within performance target
    expect(totalTime).toBeLessThan(3000); // 3 seconds for 50 conversations
    
    // UI should remain responsive during import
    await expect(page.getByRole('button', { name: 'Cancel Import' })).toBeVisible();
    
    // Performance metrics should be displayed
    await expect(page.getByText(/Processing time:/)).toBeVisible();
    await expect(page.getByText(/conversations per second/)).toBeVisible();
  });
});