<script>
  import { onMount } from 'svelte';
  
  let conversations = [];
  let loading = true;
  let error = null;
  let page = 1;
  let hasMore = true;
  let selectedIndex = -1;
  
  // Filters
  let filterProvider = '';
  let filterDateFrom = '';
  let filterDateTo = '';
  
  async function loadConversations() {
    loading = true;
    error = null;
    
    try {
      const params = new URLSearchParams({
        page: page.toString(),
        limit: '50'
      });
      
      if (filterProvider) params.append('provider', filterProvider);
      if (filterDateFrom) params.append('date_from', filterDateFrom);
      if (filterDateTo) params.append('date_to', filterDateTo);
      
      const response = await fetch(`/api/conversations?${params}`);
      if (response.ok) {
        const data = await response.json();
        conversations = data.conversations || [];
        hasMore = data.has_more || false;
        selectedIndex = -1;
      } else {
        error = 'Failed to load conversations';
      }
    } catch (err) {
      error = 'Error loading conversations: ' + err.message;
    } finally {
      loading = false;
    }
  }
  
  function applyFilters() {
    page = 1;
    loadConversations();
  }
  
  function nextPage() {
    if (hasMore) {
      page += 1;
      loadConversations();
    }
  }
  
  function prevPage() {
    if (page > 1) {
      page -= 1;
      loadConversations();
    }
  }
  
  // Keyboard navigation
  function handleKeydown(e) {
    if (e.key === 'j' || e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, conversations.length - 1);
      scrollToSelected();
    } else if (e.key === 'k' || e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, -1);
      scrollToSelected();
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      window.location.href = `/conversations/${conversations[selectedIndex].id}`;
    }
  }
  
  function scrollToSelected() {
    if (selectedIndex >= 0) {
      const elements = document.querySelectorAll('.conversation-item');
      elements[selectedIndex]?.scrollIntoView({ block: 'nearest' });
    }
  }
  
  onMount(() => {
    loadConversations();
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="container">
  <h1>Conversations</h1>
  
  <div class="filters">
    <div class="filter-group">
      <label for="provider">Provider:</label>
      <select id="provider" bind:value={filterProvider} on:change={applyFilters}>
        <option value="">All</option>
        <option value="chatgpt">ChatGPT</option>
        <option value="claude">Claude</option>
        <option value="gemini">Gemini</option>
        <option value="xai">XAI</option>
        <option value="zed">Zed</option>
      </select>
    </div>
    
    <div class="filter-group">
      <label for="date-from">From:</label>
      <input 
        id="date-from"
        type="date" 
        bind:value={filterDateFrom} 
        on:change={applyFilters}
      />
    </div>
    
    <div class="filter-group">
      <label for="date-to">To:</label>
      <input 
        id="date-to"
        type="date" 
        bind:value={filterDateTo} 
        on:change={applyFilters}
      />
    </div>
  </div>
  
  {#if loading}
    <div class="loading">Loading conversations...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if conversations.length === 0}
    <div class="empty">No conversations found</div>
  {:else}
    <ul class="conversation-list">
      {#each conversations as conversation, index}
        <li>
          <a 
            href="/conversations/{conversation.id}"
            class="conversation-item"
            class:selected={index === selectedIndex}
          >
            <div class="conversation-title">
              {conversation.title || 'Untitled Conversation'}
            </div>
            <div class="conversation-meta">
              {conversation.provider} • 
              {new Date(conversation.created_at).toLocaleDateString()} • 
              {conversation.message_count} messages
            </div>
          </a>
        </li>
      {/each}
    </ul>
    
    <div class="pagination">
      <button on:click={prevPage} disabled={page === 1}>
        Previous
      </button>
      <span class="page-info">Page {page}</span>
      <button on:click={nextPage} disabled={!hasMore}>
        Next
      </button>
    </div>
  {/if}
</div>

<style>
  .filters {
    display: flex;
    gap: 16px;
    margin-bottom: 24px;
    padding: 16px;
    background: var(--gray-50);
    border-radius: 4px;
  }
  
  .filter-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  
  .filter-group label {
    font-size: 13px;
    color: var(--gray-600);
  }
  
  .filter-group select,
  .filter-group input {
    padding: 6px 8px;
    font-size: 13px;
  }
  
  .conversation-item.selected {
    outline: 2px solid var(--primary);
    outline-offset: -2px;
  }
  
  .empty {
    text-align: center;
    padding: 48px;
    color: var(--gray-600);
  }
  
  .error {
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #dc2626;
    padding: 12px 16px;
    border-radius: 4px;
    margin: 16px 0;
  }
  
  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    margin-top: 32px;
    padding: 16px;
  }
  
  .page-info {
    font-size: 14px;
    color: var(--gray-600);
  }
  
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>