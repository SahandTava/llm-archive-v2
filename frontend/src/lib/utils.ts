// Utility functions for the frontend

export function debounce<T extends (...args: any[]) => any>(
    func: T,
    wait: number
): (...args: Parameters<T>) => void {
    let timeout: NodeJS.Timeout | null = null;
    
    return (...args: Parameters<T>) => {
        if (timeout) clearTimeout(timeout);
        
        timeout = setTimeout(() => {
            func(...args);
        }, wait);
    };
}

export function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
}

export function formatMessagePreview(content: string, maxLength: number = 150): string {
    if (content.length <= maxLength) return content;
    return content.substring(0, maxLength) + '...';
}

// Export conversation to different formats
export async function exportConversation(
    conversationId: number,
    format: 'markdown' | 'json' | 'academic' | 'blog' = 'markdown'
) {
    const response = await fetch(`/api/conversations/${conversationId}/export?format=${format}`);
    const blob = await response.blob();
    
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `conversation-${conversationId}.${format === 'json' ? 'json' : 'md'}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

// Advanced keyboard shortcuts
export function setupKeyboardShortcuts() {
    const shortcuts: Record<string, () => void> = {
        '/': () => document.getElementById('search-input')?.focus(),
        'Escape': () => (document.activeElement as HTMLElement)?.blur(),
        'g h': () => window.location.href = '/',
        'g c': () => window.location.href = '/conversations',
        'g s': () => window.location.href = '/stats',
        '?': () => showKeyboardHelp(),
    };
    
    let sequence = '';
    
    document.addEventListener('keydown', (e) => {
        // Don't trigger shortcuts when typing in inputs
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
            return;
        }
        
        const key = e.key;
        sequence += key;
        
        // Check for multi-key sequences
        setTimeout(() => {
            if (shortcuts[sequence]) {
                shortcuts[sequence]();
                sequence = '';
            } else if (!Object.keys(shortcuts).some(s => s.startsWith(sequence))) {
                sequence = '';
            }
        }, 300);
        
        // Single key shortcuts
        if (shortcuts[key]) {
            e.preventDefault();
            shortcuts[key]();
            sequence = '';
        }
    });
}

function showKeyboardHelp() {
    // In a real app, this would show a modal
    console.log(`
Keyboard Shortcuts:
/ - Focus search
j/k - Navigate up/down
Enter - Open selected
e - Export current conversation
g h - Go home
g c - Go to conversations
g s - Go to stats
? - Show this help
    `);
}