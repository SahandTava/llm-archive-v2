<script>
  import { createEventDispatcher } from 'svelte';
  
  export let message;
  export let index;
  export let isCollapsed = false;
  
  const dispatch = createEventDispatcher();
  
  // Format content - handle code blocks, preserve whitespace
  function formatContent(content) {
    if (!content) return '';
    
    // Simple code block detection and formatting
    return content.replace(/```(\w*)\n([\s\S]*?)```/g, (match, lang, code) => {
      return `<pre><code class="language-${lang || 'text'}">${escapeHtml(code.trim())}</code></pre>`;
    });
  }
  
  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
  
  // Determine if message is long enough to need collapsing
  $: isLong = message.content && message.content.length > 1000;
  $: displayContent = isCollapsed && isLong 
    ? message.content.substring(0, 500) + '...' 
    : message.content;
</script>

<div class="message {message.role}">
  <div class="message-header">
    <span class="message-role">{message.role}</span>
    {#if message.model}
      <span class="message-model">{message.model}</span>
    {/if}
    {#if isLong}
      <button 
        class="toggle-button"
        on:click={() => dispatch('toggle')}
      >
        {isCollapsed ? 'Expand' : 'Collapse'}
      </button>
    {/if}
  </div>
  
  <div class="message-content">
    {@html formatContent(displayContent)}
  </div>
  
  {#if message.timestamp}
    <div class="message-timestamp">
      {new Date(message.timestamp).toLocaleString()}
    </div>
  {/if}
</div>

<style>
  .message {
    padding: 16px;
    border-bottom: 1px solid var(--gray-100);
  }
  
  .message.user {
    background: var(--gray-50);
  }
  
  .message.system {
    background: #fef3c7;
  }
  
  .message-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }
  
  .message-role {
    font-size: 12px;
    font-weight: 600;
    color: var(--gray-600);
    text-transform: uppercase;
  }
  
  .message-model {
    font-size: 12px;
    color: var(--gray-500);
  }
  
  .toggle-button {
    margin-left: auto;
    font-size: 12px;
    padding: 4px 8px;
  }
  
  .message-content {
    white-space: pre-wrap;
    word-wrap: break-word;
    line-height: 1.6;
  }
  
  .message-content :global(pre) {
    background: var(--gray-900);
    color: white;
    padding: 12px;
    border-radius: 4px;
    overflow-x: auto;
    margin: 8px 0;
  }
  
  .message-content :global(code) {
    font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', monospace;
    font-size: 13px;
  }
  
  .message-timestamp {
    font-size: 11px;
    color: var(--gray-500);
    margin-top: 8px;
  }
</style>