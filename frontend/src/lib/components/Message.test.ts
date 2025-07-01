import { render, screen } from '@testing-library/svelte';
import { describe, it, expect } from 'vitest';
import Message from './Message.svelte';

describe('Message Component', () => {
  it('renders user message correctly', () => {
    render(Message, {
      props: {
        role: 'user',
        content: 'Hello, assistant!',
        timestamp: '2023-01-01T12:00:00Z',
      },
    });
    
    expect(screen.getByText('Hello, assistant!')).toBeInTheDocument();
    expect(screen.getByText('User')).toBeInTheDocument();
    expect(screen.getByText(/Jan 1, 2023/)).toBeInTheDocument();
  });
  
  it('renders assistant message correctly', () => {
    render(Message, {
      props: {
        role: 'assistant',
        content: 'Hello! How can I help you today?',
        timestamp: '2023-01-01T12:00:30Z',
        model: 'gpt-4',
      },
    });
    
    expect(screen.getByText('Hello! How can I help you today?')).toBeInTheDocument();
    expect(screen.getByText('Assistant')).toBeInTheDocument();
    expect(screen.getByText('gpt-4')).toBeInTheDocument();
  });
  
  it('renders system message correctly', () => {
    render(Message, {
      props: {
        role: 'system',
        content: 'You are a helpful assistant.',
        timestamp: '2023-01-01T12:00:00Z',
      },
    });
    
    expect(screen.getByText('You are a helpful assistant.')).toBeInTheDocument();
    expect(screen.getByText('System')).toBeInTheDocument();
  });
  
  it('handles markdown content', () => {
    render(Message, {
      props: {
        role: 'assistant',
        content: '**Bold text** and *italic text*\n\n```python\nprint("Hello")\n```',
        timestamp: '2023-01-01T12:00:00Z',
      },
    });
    
    const content = screen.getByTestId('message-content');
    expect(content.querySelector('strong')).toBeInTheDocument();
    expect(content.querySelector('em')).toBeInTheDocument();
    expect(content.querySelector('pre')).toBeInTheDocument();
  });
  
  it('applies correct styling for different roles', () => {
    const { container: userContainer } = render(Message, {
      props: {
        role: 'user',
        content: 'User message',
        timestamp: '2023-01-01T12:00:00Z',
      },
    });
    
    const { container: assistantContainer } = render(Message, {
      props: {
        role: 'assistant',
        content: 'Assistant message',
        timestamp: '2023-01-01T12:00:00Z',
      },
    });
    
    const userMessage = userContainer.querySelector('[data-role="user"]');
    const assistantMessage = assistantContainer.querySelector('[data-role="assistant"]');
    
    expect(userMessage).toHaveClass('message-user');
    expect(assistantMessage).toHaveClass('message-assistant');
  });
  
  it('handles long messages efficiently', () => {
    const longContent = 'Lorem ipsum '.repeat(1000);
    
    const { container } = render(Message, {
      props: {
        role: 'user',
        content: longContent,
        timestamp: '2023-01-01T12:00:00Z',
      },
    });
    
    const messageElement = container.querySelector('.message');
    expect(messageElement).toHaveStyle({ maxHeight: '600px', overflow: 'auto' });
  });
});