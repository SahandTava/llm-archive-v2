<script lang="ts">
    import { onMount } from 'svelte';
    import { searchQuery, searchResults, isSearching, selectedIndex, navigateUp, navigateDown } from '$lib/stores/search';
    import { goto } from '$app/navigation';
    import { setupKeyboardShortcuts } from '$lib/utils';

    let searchInput: HTMLInputElement;

    onMount(() => {
        setupKeyboardShortcuts();
        searchInput?.focus();
    });

    function handleKeydown(e: KeyboardEvent) {
        switch(e.key) {
            case 'ArrowDown':
            case 'j':
                e.preventDefault();
                navigateDown();
                break;
            case 'ArrowUp':
            case 'k':
                e.preventDefault();
                navigateUp();
                break;
            case 'Enter':
                e.preventDefault();
                const results = $searchResults;
                if (results[$selectedIndex]) {
                    goto(`/conversations/${results[$selectedIndex].conversation_id}`);
                }
                break;
        }
    }

    // Real-time search preview
    $: searchPreview = $searchQuery.length > 0 && $searchQuery.length < 3 
        ? 'Type at least 3 characters to search...' 
        : '';
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="search-container">
    <h1>LLM Archive Search</h1>
    
    <div class="search-box">
        <input
            bind:this={searchInput}
            bind:value={$searchQuery}
            type="search"
            id="search-input"
            placeholder="Search conversations... (try 'provider:chatgpt' or 'after:2024-01-01')"
            autocomplete="off"
            spellcheck="false"
        />
        
        {#if $isSearching}
            <div class="search-indicator">Searching...</div>
        {/if}
        
        {#if searchPreview}
            <div class="search-preview">{searchPreview}</div>
        {/if}
    </div>

    <div class="search-tips">
        <details>
            <summary>Advanced Search</summary>
            <ul>
                <li><code>provider:chatgpt</code> - Filter by provider</li>
                <li><code>role:user</code> - Show only user messages</li>
                <li><code>after:2024-01-01</code> - Messages after date</li>
                <li><code>before:2024-12-31</code> - Messages before date</li>
                <li>Combine filters: <code>rust provider:claude role:assistant</code></li>
            </ul>
        </details>
    </div>

    {#if $searchResults.length > 0}
        <div class="search-results" role="listbox">
            {#each $searchResults as result, i}
                <a 
                    href="/conversations/{result.conversation_id}"
                    class="search-result"
                    class:selected={i === $selectedIndex}
                    role="option"
                    aria-selected={i === $selectedIndex}
                >
                    <h3>{result.title}</h3>
                    <p class="snippet">{@html result.snippet}</p>
                    <div class="meta">
                        Score: {result.score.toFixed(2)}
                    </div>
                </a>
            {/each}
        </div>
    {/if}
</div>

<style>
    .search-container {
        max-width: 800px;
        margin: 0 auto;
        padding: 2rem;
    }

    h1 {
        text-align: center;
        margin-bottom: 2rem;
    }

    .search-box {
        position: relative;
        margin-bottom: 1rem;
    }

    input[type="search"] {
        width: 100%;
        padding: 1rem;
        font-size: 1.2rem;
        border: 2px solid #ddd;
        border-radius: 8px;
        transition: border-color 0.2s;
    }

    input[type="search"]:focus {
        outline: none;
        border-color: #007bff;
    }

    .search-indicator {
        position: absolute;
        right: 1rem;
        top: 50%;
        transform: translateY(-50%);
        color: #666;
        font-size: 0.9rem;
    }

    .search-preview {
        margin-top: 0.5rem;
        color: #666;
        font-size: 0.9rem;
    }

    .search-tips {
        margin-bottom: 2rem;
    }

    .search-tips details {
        background: #f5f5f5;
        padding: 0.5rem 1rem;
        border-radius: 4px;
    }

    .search-tips summary {
        cursor: pointer;
        font-weight: 500;
    }

    .search-tips ul {
        margin-top: 0.5rem;
        padding-left: 1.5rem;
    }

    .search-tips code {
        background: #e0e0e0;
        padding: 0.2rem 0.4rem;
        border-radius: 3px;
        font-size: 0.9rem;
    }

    .search-results {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .search-result {
        display: block;
        padding: 1rem;
        border: 1px solid #ddd;
        border-radius: 8px;
        text-decoration: none;
        color: inherit;
        transition: all 0.2s;
    }

    .search-result:hover,
    .search-result.selected {
        border-color: #007bff;
        background: #f0f7ff;
    }

    .search-result h3 {
        margin: 0 0 0.5rem 0;
        color: #333;
    }

    .search-result .snippet {
        margin: 0 0 0.5rem 0;
        color: #666;
        line-height: 1.5;
    }

    .search-result .snippet :global(mark) {
        background: #ffeb3b;
        padding: 0.1rem;
        border-radius: 2px;
    }

    .search-result .meta {
        font-size: 0.8rem;
        color: #999;
    }
</style>