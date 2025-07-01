// Integration tests for API endpoints
// Tests the frontend's integration with the backend API

import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { spawn, ChildProcessWithoutNullStreams } from 'child_process';
import fetch from 'node-fetch';

interface Conversation {
  id: string;
  title: string;
  provider: string;
  start_time: string;
  end_time: string;
  message_count: number;
  messages?: Message[];
}

interface Message {
  id: string;
  role: string;
  content: string;
  timestamp: string;
  model?: string;
}

interface ApiResponse<T> {
  data?: T;
  error?: string;
}

interface ConversationsResponse {
  conversations: Conversation[];
  total: number;
  page: number;
  total_pages: number;
}

describe('API Integration Tests', () => {
  let backendProcess: ChildProcessWithoutNullStreams;
  const BASE_URL = 'http://localhost:3001';
  const TIMEOUT = 30000;

  beforeAll(async () => {
    // Start backend server for testing
    backendProcess = spawn('../backend/target/debug/llm-archive-backend', ['--port', '3001'], {
      cwd: process.cwd(),
      stdio: 'pipe'
    });

    // Wait for server to start
    let attempts = 0;
    const maxAttempts = 30;
    
    while (attempts < maxAttempts) {
      try {
        const response = await fetch(`${BASE_URL}/health`);
        if (response.ok) {
          break;
        }
      } catch (error) {
        // Server not ready yet
      }
      
      await new Promise(resolve => setTimeout(resolve, 1000));
      attempts++;
    }

    if (attempts >= maxAttempts) {
      throw new Error('Backend server failed to start within timeout');
    }
  }, TIMEOUT);

  afterAll(async () => {
    if (backendProcess) {
      backendProcess.kill();
    }
  });

  beforeEach(async () => {
    // Reset database state for each test
    await fetch(`${BASE_URL}/api/test/reset`, { method: 'POST' });
  });

  describe('Health Check', () => {
    it('should return healthy status', async () => {
      const response = await fetch(`${BASE_URL}/health`);
      expect(response.ok).toBe(true);
      
      const data = await response.json();
      expect(data.status).toBe('healthy');
    });
  });

  describe('Conversations API', () => {
    beforeEach(async () => {
      // Seed test data
      const testConversations = [
        {
          title: 'Test ChatGPT Conversation',
          provider: 'chatgpt',
          messages: [
            { role: 'user', content: 'Hello' },
            { role: 'assistant', content: 'Hi there!', model: 'gpt-4' }
          ]
        },
        {
          title: 'Test Claude Conversation', 
          provider: 'claude',
          messages: [
            { role: 'user', content: 'Help me code' },
            { role: 'assistant', content: 'I can help with coding!' }
          ]
        }
      ];

      await fetch(`${BASE_URL}/api/test/seed`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ conversations: testConversations })
      });
    });

    it('should fetch conversations list', async () => {
      const response = await fetch(`${BASE_URL}/api/conversations`);
      expect(response.ok).toBe(true);
      
      const data: ConversationsResponse = await response.json();
      expect(data.conversations).toHaveLength(2);
      expect(data.total).toBe(2);
      expect(data.page).toBe(1);
      
      // Check conversation structure
      const conv = data.conversations[0];
      expect(conv).toHaveProperty('id');
      expect(conv).toHaveProperty('title');
      expect(conv).toHaveProperty('provider');
      expect(conv).toHaveProperty('start_time');
      expect(conv).toHaveProperty('end_time');
      expect(conv).toHaveProperty('message_count');
    });

    it('should handle pagination correctly', async () => {
      // Add more conversations to test pagination
      const moreConversations = Array.from({ length: 25 }, (_, i) => ({
        title: `Conversation ${i + 3}`,
        provider: 'chatgpt',
        messages: [
          { role: 'user', content: `Message ${i}` },
          { role: 'assistant', content: `Response ${i}` }
        ]
      }));

      await fetch(`${BASE_URL}/api/test/seed`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ conversations: moreConversations })
      });

      // Test first page
      const page1 = await fetch(`${BASE_URL}/api/conversations?page=1&per_page=10`);
      const data1: ConversationsResponse = await page1.json();
      
      expect(data1.conversations).toHaveLength(10);
      expect(data1.total).toBe(27);
      expect(data1.page).toBe(1);
      expect(data1.total_pages).toBe(3);

      // Test second page
      const page2 = await fetch(`${BASE_URL}/api/conversations?page=2&per_page=10`);
      const data2: ConversationsResponse = await page2.json();
      
      expect(data2.conversations).toHaveLength(10);
      expect(data2.page).toBe(2);
      
      // Ensure different conversations on different pages
      const page1Ids = data1.conversations.map(c => c.id);
      const page2Ids = data2.conversations.map(c => c.id);
      expect(page1Ids).not.toEqual(page2Ids);
    });

    it('should filter by provider', async () => {
      const response = await fetch(`${BASE_URL}/api/conversations?provider=chatgpt`);
      const data: ConversationsResponse = await response.json();
      
      expect(data.conversations).toHaveLength(1);
      expect(data.conversations[0].provider).toBe('chatgpt');
    });

    it('should filter by date range', async () => {
      const startDate = '2023-01-01';
      const endDate = '2023-12-31';
      
      const response = await fetch(
        `${BASE_URL}/api/conversations?start_date=${startDate}&end_date=${endDate}`
      );
      
      expect(response.ok).toBe(true);
      const data: ConversationsResponse = await response.json();
      
      // All conversations should be within date range
      for (const conv of data.conversations) {
        const convDate = new Date(conv.start_time);
        expect(convDate.getTime()).toBeGreaterThanOrEqual(new Date(startDate).getTime());
        expect(convDate.getTime()).toBeLessThanOrEqual(new Date(endDate).getTime());
      }
    });

    it('should handle search queries', async () => {
      const response = await fetch(`${BASE_URL}/api/conversations?q=ChatGPT`);
      const data: ConversationsResponse = await response.json();
      
      expect(data.conversations).toHaveLength(1);
      expect(data.conversations[0].title).toContain('ChatGPT');
    });

    it('should fetch individual conversation', async () => {
      // First get list to get an ID
      const listResponse = await fetch(`${BASE_URL}/api/conversations`);
      const listData: ConversationsResponse = await listResponse.json();
      
      const conversationId = listData.conversations[0].id;
      
      // Fetch individual conversation
      const response = await fetch(`${BASE_URL}/api/conversations/${conversationId}`);
      expect(response.ok).toBe(true);
      
      const conversation: Conversation = await response.json();
      expect(conversation.id).toBe(conversationId);
      expect(conversation.messages).toBeDefined();
      expect(conversation.messages!.length).toBeGreaterThan(0);
      
      // Check message structure
      const message = conversation.messages![0];
      expect(message).toHaveProperty('id');
      expect(message).toHaveProperty('role');
      expect(message).toHaveProperty('content');
      expect(message).toHaveProperty('timestamp');
    });

    it('should return 404 for non-existent conversation', async () => {
      const response = await fetch(`${BASE_URL}/api/conversations/non-existent-id`);
      expect(response.status).toBe(404);
    });
  });

  describe('Search API', () => {
    beforeEach(async () => {
      // Seed conversations with searchable content
      const searchableConversations = [
        {
          title: 'Machine Learning Discussion',
          provider: 'chatgpt',
          messages: [
            { role: 'user', content: 'Explain machine learning algorithms' },
            { role: 'assistant', content: 'Machine learning involves training models on data to make predictions' }
          ]
        },
        {
          title: 'Python Programming Help',
          provider: 'claude',
          messages: [
            { role: 'user', content: 'How do I use pandas in Python?' },
            { role: 'assistant', content: 'Pandas is a powerful data manipulation library in Python' }
          ]
        }
      ];

      await fetch(`${BASE_URL}/api/test/seed`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ conversations: searchableConversations })
      });
    });

    it('should perform full-text search', async () => {
      const response = await fetch(`${BASE_URL}/api/search?q=machine%20learning`);
      expect(response.ok).toBe(true);
      
      const data = await response.json();
      expect(data.results).toHaveLength(1);
      expect(data.results[0].title).toContain('Machine Learning');
    });

    it('should search in message content', async () => {
      const response = await fetch(`${BASE_URL}/api/search?q=pandas`);
      const data = await response.json();
      
      expect(data.results).toHaveLength(1);
      expect(data.results[0].title).toContain('Python');
    });

    it('should handle empty search results', async () => {
      const response = await fetch(`${BASE_URL}/api/search?q=nonexistentterm`);
      const data = await response.json();
      
      expect(data.results).toHaveLength(0);
      expect(data.total).toBe(0);
    });

    it('should validate search performance', async () => {
      const startTime = Date.now();
      
      await fetch(`${BASE_URL}/api/search?q=machine`);
      
      const endTime = Date.now();
      const duration = endTime - startTime;
      
      // Search should complete in under 100ms
      expect(duration).toBeLessThan(100);
    });
  });

  describe('Import API', () => {
    it('should accept file upload', async () => {
      const formData = new FormData();
      const testFile = new Blob([JSON.stringify([{
        title: 'Uploaded Conversation',
        messages: [
          { role: 'user', content: 'Test upload' },
          { role: 'assistant', content: 'Upload successful' }
        ]
      }])], { type: 'application/json' });
      
      formData.append('file', testFile, 'test.json');
      formData.append('provider', 'chatgpt');

      const response = await fetch(`${BASE_URL}/api/import`, {
        method: 'POST',
        body: formData
      });

      expect(response.ok).toBe(true);
      
      const result = await response.json();
      expect(result.imported_count).toBe(1);
      expect(result.errors).toHaveLength(0);
    });

    it('should validate file format', async () => {
      const formData = new FormData();
      const invalidFile = new Blob(['invalid json'], { type: 'application/json' });
      
      formData.append('file', invalidFile, 'invalid.json');
      formData.append('provider', 'chatgpt');

      const response = await fetch(`${BASE_URL}/api/import`, {
        method: 'POST',
        body: formData
      });

      expect(response.status).toBe(400);
      
      const result = await response.json();
      expect(result.error).toContain('Invalid');
    });

    it('should handle import performance requirements', async () => {
      // Create large test file (100 conversations)
      const largeData = Array.from({ length: 100 }, (_, i) => ({
        title: `Bulk Import Conversation ${i}`,
        messages: [
          { role: 'user', content: `Test message ${i}` },
          { role: 'assistant', content: `Response ${i}` }
        ]
      }));

      const formData = new FormData();
      const testFile = new Blob([JSON.stringify(largeData)], { type: 'application/json' });
      formData.append('file', testFile, 'bulk.json');
      formData.append('provider', 'chatgpt');

      const startTime = Date.now();
      
      const response = await fetch(`${BASE_URL}/api/import`, {
        method: 'POST',
        body: formData
      });

      const endTime = Date.now();
      const duration = endTime - startTime;

      expect(response.ok).toBe(true);
      
      const result = await response.json();
      expect(result.imported_count).toBe(100);
      
      // Import should complete in under 1 second for 100 conversations
      expect(duration).toBeLessThan(1000);
    });
  });

  describe('Export API', () => {
    beforeEach(async () => {
      // Seed data for export tests
      const exportConversations = [
        {
          title: 'Export Test 1',
          provider: 'chatgpt',
          messages: [
            { role: 'user', content: 'Export test message' },
            { role: 'assistant', content: 'This will be exported' }
          ]
        }
      ];

      await fetch(`${BASE_URL}/api/test/seed`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ conversations: exportConversations })
      });
    });

    it('should export conversations as JSON', async () => {
      const response = await fetch(`${BASE_URL}/api/export?format=json`);
      expect(response.ok).toBe(true);
      expect(response.headers.get('content-type')).toContain('application/json');
      
      const data = await response.json();
      expect(Array.isArray(data)).toBe(true);
      expect(data.length).toBeGreaterThan(0);
    });

    it('should export conversations as CSV', async () => {
      const response = await fetch(`${BASE_URL}/api/export?format=csv`);
      expect(response.ok).toBe(true);
      expect(response.headers.get('content-type')).toContain('text/csv');
      
      const csvData = await response.text();
      expect(csvData).toContain('title,provider,start_time');
    });

    it('should export specific conversations', async () => {
      // Get conversation IDs
      const listResponse = await fetch(`${BASE_URL}/api/conversations`);
      const listData: ConversationsResponse = await listResponse.json();
      
      const conversationId = listData.conversations[0].id;
      
      const response = await fetch(`${BASE_URL}/api/export?format=json&ids=${conversationId}`);
      const data = await response.json();
      
      expect(data).toHaveLength(1);
      expect(data[0].id).toBe(conversationId);
    });
  });

  describe('Statistics API', () => {
    beforeEach(async () => {
      // Seed varied data for statistics
      const statsConversations = Array.from({ length: 10 }, (_, i) => ({
        title: `Stats Test ${i}`,
        provider: ['chatgpt', 'claude', 'gemini'][i % 3],
        messages: Array.from({ length: (i % 5) + 1 }, (_, j) => ({
          role: j % 2 === 0 ? 'user' : 'assistant',
          content: `Message ${j} in conversation ${i}`
        }))
      }));

      await fetch(`${BASE_URL}/api/test/seed`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ conversations: statsConversations })
      });
    });

    it('should return overall statistics', async () => {
      const response = await fetch(`${BASE_URL}/api/stats`);
      expect(response.ok).toBe(true);
      
      const stats = await response.json();
      
      expect(stats).toHaveProperty('total_conversations');
      expect(stats).toHaveProperty('total_messages');
      expect(stats).toHaveProperty('providers');
      expect(stats.total_conversations).toBeGreaterThan(0);
      expect(stats.total_messages).toBeGreaterThan(0);
      expect(Object.keys(stats.providers).length).toBeGreaterThan(0);
    });

    it('should calculate statistics performance efficiently', async () => {
      const startTime = Date.now();
      
      await fetch(`${BASE_URL}/api/stats`);
      
      const endTime = Date.now();
      const duration = endTime - startTime;
      
      // Statistics should load in under 50ms
      expect(duration).toBeLessThan(50);
    });
  });

  describe('Error Handling', () => {
    it('should handle malformed requests gracefully', async () => {
      const response = await fetch(`${BASE_URL}/api/conversations`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: 'invalid json'
      });
      
      expect(response.status).toBe(400);
      
      const error = await response.json();
      expect(error).toHaveProperty('error');
    });

    it('should return proper error codes', async () => {
      // Test 404
      const notFound = await fetch(`${BASE_URL}/api/nonexistent`);
      expect(notFound.status).toBe(404);
      
      // Test method not allowed
      const methodNotAllowed = await fetch(`${BASE_URL}/api/conversations`, {
        method: 'DELETE'
      });
      expect(methodNotAllowed.status).toBe(405);
    });

    it('should handle rate limiting', async () => {
      // Make many rapid requests
      const requests = Array.from({ length: 100 }, () => 
        fetch(`${BASE_URL}/api/conversations`)
      );
      
      const responses = await Promise.all(requests);
      
      // Should handle all requests or return 429 (rate limited)
      for (const response of responses) {
        expect([200, 429]).toContain(response.status);
      }
    });
  });
});