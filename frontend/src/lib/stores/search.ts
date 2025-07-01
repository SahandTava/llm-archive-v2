import { writable, derived } from 'svelte/store';
import { debounce } from '$lib/utils';

// Search state management with debouncing
export const searchQuery = writable('');
export const searchResults = writable<SearchResult[]>([]);
export const isSearching = writable(false);
export const selectedIndex = writable(0);

interface SearchResult {
    conversation_id: number;
    title: string;
    snippet: string;
    score: number;
}

// Debounced search function
const performSearch = debounce(async (query: string) => {
    if (query.length === 0) {
        searchResults.set([]);
        return;
    }

    isSearching.set(true);
    
    try {
        const response = await fetch(`/api/search?q=${encodeURIComponent(query)}`);
        const data = await response.json();
        searchResults.set(data.results);
        selectedIndex.set(0);
    } catch (error) {
        console.error('Search failed:', error);
        searchResults.set([]);
    } finally {
        isSearching.set(false);
    }
}, 150); // 150ms debounce for incremental search

// Subscribe to query changes
searchQuery.subscribe(query => {
    performSearch(query);
});

// Keyboard navigation
export function navigateUp() {
    selectedIndex.update(n => Math.max(0, n - 1));
}

export function navigateDown() {
    searchResults.subscribe(results => {
        selectedIndex.update(n => Math.min(results.length - 1, n + 1));
    })();
}

// Advanced search DSL
export const advancedSearchQuery = writable('');
export const searchFilters = writable({
    provider: '',
    role: '',
    afterDate: '',
    beforeDate: ''
});

// Build DSL query from filters
export const dslQuery = derived(
    [advancedSearchQuery, searchFilters],
    ([$query, $filters]) => {
        let parts = [];
        
        if ($query) parts.push($query);
        if ($filters.provider) parts.push(`provider:${$filters.provider}`);
        if ($filters.role) parts.push(`role:${$filters.role}`);
        if ($filters.afterDate) parts.push(`after:${$filters.afterDate}`);
        if ($filters.beforeDate) parts.push(`before:${$filters.beforeDate}`);
        
        return parts.join(' ');
    }
);