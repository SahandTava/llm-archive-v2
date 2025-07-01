<script>
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import Message from '$lib/components/Message.svelte';
  
  let conversation = null;
  let messages = [];
  let loading = true;
  let error = null;
  let collapsed = new Set();
  
  async function loadConversation() {
    const id = $page.params.id;
    loading = true;
    error = null;
    
    try {
      const response = await fetch(`/api/conversations/${id}`);
      if (response.ok) {
        const data = await response.json();
        conversation = data.conversation;
        messages = data.messages || [];
      } else {
        error = 'Failed to load conversation';
      }
    } catch (err) {
      error = 'Error loading conversation: ' + err.message;
    } finally {
      loading = false;
    }
  }
  
  function toggleMessage(index) {
    if (collapsed.has(index)) {
      collapsed.delete(index);
    } else {
      collapsed.add(index);
    }
    collapsed = collapsed; // Trigger reactivity
  }
  
  async function exportConversation(format) {
    const id = $page.params.id;
    try {
      const response = await fetch(`/api/conversations/${id}/export?format=${format}`);
      if (response.ok) {
        const blob = await response.blob();
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `conversation-${id}.${format}`;
        a.click();
        URL.revokeObjectURL(url);
      }
    } catch (err) {
      console.error('Export error:', err);
    }
  }
  
  // Keyboard shortcuts
  function handleKeydown(e) {
    if (e.key === 'e' && !['INPUT', 'TEXTAREA'].includes(e.target.tagName)) {
      e.preventDefault();
      // Toggle export menu or export as markdown by default
      exportConversation('md');
    }
  }
  
  onMount(() => {
    loadConversation();
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="container">
  {#if loading}
    <div class="loading">Loading conversation...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if conversation}
    <div class="conversation-header">
      <h1>{conversation.title || 'Untitled Conversation'}</h1>
      <div class="conversation-info">
        <span>{conversation.provider}</span>
        <span>•</span>
        <span>{new Date(conversation.created_at).toLocaleDateString()}</span>
        <span>•</span>
        <span>{messages.length} messages</span>
      </div>
      <div class="actions">
        <button on:click={() => exportConversation('md')}>Export as Markdown</button>
        <button on:click={() => exportConversation('json')}>Export as JSON</button>
        <button on:click={() => exportConversation('txt')}>Export as Text</button>
      </div>
    </div>
    
    <div class="messages">
      {#each messages as message, index}
        <Message 
          {message} 
          {index}
          isCollapsed={collapsed.has(index)}
          on:toggle={() => toggleMessage(index)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .conversation-header {
    padding: 24px 0;
    border-bottom: 1px solid var(--gray-200);
    margin-bottom: 24px;
  }
  
  .conversation-info {
    display: flex;
    gap: 8px;
    color: var(--gray-600);
    font-size: 14px;
    margin-bottom: 16px;
  }
  
  .actions {
    display: flex;
    gap: 8px;
  }
  
  .actions button {
    font-size: 13px;
    padding: 6px 12px;
  }
  
  .messages {
    margin-bottom: 48px;
  }
  
  .error {
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #dc2626;
    padding: 12px 16px;
    border-radius: 4px;
    margin: 16px 0;
  }
</style>