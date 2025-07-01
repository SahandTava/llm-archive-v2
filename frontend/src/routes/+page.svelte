<script>
  import { onMount } from 'svelte';
  
  let searchQuery = '';
  let results = [];
  let searching = false;
  let searchTimer;
  let selectedIndex = -1;
  
  // Debounced search function
  function handleSearch() {
    clearTimeout(searchTimer);
    if (!searchQuery.trim()) {
      results = [];
      return;
    }
    
    searching = true;
    searchTimer = setTimeout(async () => {
      try {
        const response = await fetch(`/api/search?q=${encodeURIComponent(searchQuery)}`);
        if (response.ok) {
          const data = await response.json();
          results = data.results || [];
          selectedIndex = -1;
        }
      } catch (error) {
        console.error('Search error:', error);
      } finally {
        searching = false;
      }
    }, 100); // 100ms debounce for fast typing
  }
  
  // Keyboard navigation
  function handleKeydown(e) {
    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, -1);
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      window.location.href = `/conversations/${results[selectedIndex].conversation_id}`;
    }
  }
  
  // Highlight search terms in text
  function highlightTerms(text, query) {
    if (!query) return text;
    const terms = query.split(/\s+/).filter(Boolean);
    let highlighted = text;
    terms.forEach(term => {
      const regex = new RegExp(`(${term})`, 'gi');
      highlighted = highlighted.replace(regex, '<mark>$1</mark>');
    });
    return highlighted;
  }
  
  onMount(() => {
    // Auto-focus search on mount
    const searchInput = document.querySelector('#global-search');
    if (searchInput) searchInput.focus();
  });
</script>

<div class="search-bar">
  <div class="container">
    <input
      id="global-search"
      type="search"
      class="search-input"
      placeholder="Search your conversations..."
      bind:value={searchQuery}
      on:input={handleSearch}
      on:keydown={handleKeydown}
      autocomplete="off"
      spellcheck="false"
    />
  </div>
</div>

<div class="container">
  {#if searching}
    <div class="loading">Searching...</div>
  {:else if results.length > 0}
    <div class="results-count">{results.length} results</div>
    <div class="search-results">
      {#each results as result, index}
        <a 
          href="/conversations/{result.conversation_id}"
          class="result-item"
          class:selected={index === selectedIndex}
        >
          <div class="result-title">{result.title || 'Untitled Conversation'}</div>
          <div class="result-snippet">
            {@html highlightTerms(result.snippet, searchQuery)}
          </div>
          <div class="result-meta">
            {result.provider} • {new Date(result.created_at).toLocaleDateString()}
          </div>
        </a>
      {/each}
    </div>
  {:else if searchQuery}
    <div class="no-results">No results found for "{searchQuery}"</div>
  {:else}
    <div class="welcome">
      <h1>LLM Archive Search</h1>
      <p>Start typing to search through your AI conversations</p>
      <div class="shortcuts">
        <h3>Keyboard Shortcuts</h3>
        <ul>
          <li><kbd>/</kbd> Focus search</li>
          <li><kbd>↓</kbd> or <kbd>j</kbd> Next result</li>
          <li><kbd>↑</kbd> or <kbd>k</kbd> Previous result</li>
          <li><kbd>Enter</kbd> Open selected result</li>
        </ul>
      </div>
    </div>
  {/if}
</div>

<style>
  .results-count {
    margin: 16px 0 8px;
    color: var(--gray-600);
    font-size: 13px;
  }
  
  .search-results {
    margin-top: 16px;
  }
  
  .result-item {
    display: block;
    padding: 16px;
    border: 1px solid var(--gray-200);
    border-radius: 4px;
    margin-bottom: 8px;
    color: inherit;
    text-decoration: none;
  }
  
  .result-item:hover,
  .result-item.selected {
    background: var(--gray-50);
    border-color: var(--gray-300);
  }
  
  .result-item.selected {
    outline: 2px solid var(--primary);
    outline-offset: -2px;
  }
  
  .result-title {
    font-weight: 600;
    margin-bottom: 4px;
  }
  
  .result-snippet {
    color: var(--gray-700);
    margin-bottom: 8px;
    line-height: 1.6;
  }
  
  .result-snippet :global(mark) {
    background: #fef3c7;
    padding: 1px 2px;
    border-radius: 2px;
  }
  
  .result-meta {
    font-size: 12px;
    color: var(--gray-500);
  }
  
  .no-results {
    text-align: center;
    padding: 48px;
    color: var(--gray-600);
  }
  
  .welcome {
    padding: 48px 0;
  }
  
  .welcome p {
    color: var(--gray-600);
    margin-bottom: 32px;
  }
  
  .shortcuts {
    background: var(--gray-50);
    padding: 20px;
    border-radius: 4px;
    max-width: 400px;
  }
  
  .shortcuts ul {
    list-style: none;
  }
  
  .shortcuts li {
    margin: 8px 0;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  
  kbd {
    background: white;
    border: 1px solid var(--gray-300);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: monospace;
    font-size: 12px;
    box-shadow: 0 1px 0 var(--gray-200);
  }
</style>