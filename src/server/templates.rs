use askama::Template;
use crate::models::{Conversation, Message, SearchResult};
use super::Stats;

/// Base template for all pages
#[derive(Template)]
#[template(path = "base.html")]
struct BaseTemplate<'a> {
    title: &'a str,
    content: &'a str,
}

/// Index page template
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    stats: &'a Stats,
}

/// Search results template
#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate<'a> {
    query: &'a str,
    results: &'a [SearchResult],
}

/// Conversation view template
#[derive(Template)]
#[template(path = "conversation.html")]
struct ConversationTemplate<'a> {
    conversation: &'a Conversation,
    messages: &'a [Message],
}

/// Render index page
pub fn render_index(stats: &Stats) -> anyhow::Result<String> {
    // For now, return a simple HTML string
    // TODO: Implement proper Askama templates
    Ok(format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>LLM Archive</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #f5f5f5;
        }}
        .stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }}
        .stat-card {{
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .stat-value {{
            font-size: 2em;
            font-weight: bold;
            color: #333;
        }}
        .stat-label {{
            color: #666;
            margin-top: 5px;
        }}
        .search-box {{
            width: 100%;
            padding: 15px;
            font-size: 18px;
            border: 1px solid #ddd;
            border-radius: 8px;
            margin: 20px 0;
        }}
        .search-box:focus {{
            outline: none;
            border-color: #007bff;
        }}
    </style>
</head>
<body>
    <h1>LLM Archive V2</h1>
    
    <form action="/search" method="get">
        <input type="text" name="q" class="search-box" placeholder="Search conversations..." autofocus>
    </form>
    
    <div class="stats">
        <div class="stat-card">
            <div class="stat-value">{}</div>
            <div class="stat-label">Total Conversations</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{}</div>
            <div class="stat-label">Total Messages</div>
        </div>
        {}
    </div>
    
    <div style="margin-top: 40px; color: #666;">
        <p>Keyboard shortcuts:</p>
        <ul>
            <li><kbd>/</kbd> - Focus search</li>
            <li><kbd>↓</kbd> <kbd>↑</kbd> - Navigate results</li>
            <li><kbd>Enter</kbd> - Open conversation</li>
            <li><kbd>Esc</kbd> - Clear search</li>
        </ul>
    </div>
    
    <script>
        // Keyboard navigation
        document.addEventListener('keydown', (e) => {{
            if (e.key === '/' && !e.target.matches('input')) {{
                e.preventDefault();
                document.querySelector('.search-box').focus();
            }}
        }});
    </script>
</body>
</html>
    "#,
        stats.total_conversations,
        stats.total_messages,
        stats.providers.iter()
            .map(|p| format!(
                r#"<div class="stat-card">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">{}</div>
                </div>"#,
                p.count, p.name
            ))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

/// Render search results
pub fn render_search_results(query: &str, results: &[SearchResult]) -> anyhow::Result<String> {
    Ok(format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Search: {} - LLM Archive</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #f5f5f5;
        }}
        .search-header {{
            margin-bottom: 20px;
        }}
        .search-box {{
            width: 100%;
            padding: 12px;
            font-size: 16px;
            border: 1px solid #ddd;
            border-radius: 8px;
        }}
        .results {{
            margin-top: 20px;
        }}
        .result {{
            background: white;
            padding: 20px;
            margin-bottom: 10px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            cursor: pointer;
            transition: all 0.2s;
        }}
        .result:hover {{
            box-shadow: 0 4px 8px rgba(0,0,0,0.15);
            transform: translateY(-1px);
        }}
        .result.selected {{
            border: 2px solid #007bff;
        }}
        .result-title {{
            font-size: 18px;
            font-weight: bold;
            color: #333;
            margin-bottom: 5px;
        }}
        .result-meta {{
            color: #666;
            font-size: 14px;
            margin-bottom: 10px;
        }}
        .result-snippet {{
            color: #444;
            line-height: 1.5;
        }}
        .result-snippet mark {{
            background: #ffeb3b;
            padding: 2px;
        }}
        .no-results {{
            text-align: center;
            padding: 40px;
            color: #666;
        }}
    </style>
</head>
<body>
    <div class="search-header">
        <h1><a href="/" style="text-decoration: none; color: inherit;">LLM Archive</a></h1>
        <form action="/search" method="get">
            <input type="text" name="q" value="{}" class="search-box" autofocus>
        </form>
    </div>
    
    <div class="results">
        {}
    </div>
    
    <script>
        let selectedIndex = -1;
        const results = document.querySelectorAll('.result');
        
        // Click handler
        results.forEach((result, index) => {{
            result.addEventListener('click', () => {{
                window.location.href = result.dataset.href;
            }});
        }});
        
        // Keyboard navigation
        document.addEventListener('keydown', (e) => {{
            if (e.key === 'ArrowDown') {{
                e.preventDefault();
                selectResult(selectedIndex + 1);
            }} else if (e.key === 'ArrowUp') {{
                e.preventDefault();
                selectResult(selectedIndex - 1);
            }} else if (e.key === 'Enter' && selectedIndex >= 0) {{
                e.preventDefault();
                window.location.href = results[selectedIndex].dataset.href;
            }} else if (e.key === 'Escape') {{
                e.preventDefault();
                document.querySelector('.search-box').value = '';
                document.querySelector('.search-box').focus();
            }}
        }});
        
        function selectResult(index) {{
            if (index < 0) index = 0;
            if (index >= results.length) index = results.length - 1;
            
            results.forEach(r => r.classList.remove('selected'));
            if (index >= 0 && index < results.length) {{
                results[index].classList.add('selected');
                results[index].scrollIntoView({{ block: 'nearest' }});
            }}
            selectedIndex = index;
        }}
    </script>
</body>
</html>
    "#,
        query,
        query,
        if results.is_empty() {
            format!(r#"<div class="no-results">No results found for "{}"</div>"#, query)
        } else {
            results.iter().enumerate()
                .map(|(i, r)| format!(
                    r#"<div class="result" data-href="/conversation/{}" data-index="{}">
                        <div class="result-title">{}</div>
                        <div class="result-meta">{} • {} • {}</div>
                        <div class="result-snippet">{}</div>
                    </div>"#,
                    r.conversation.id,
                    i,
                    r.conversation.title.as_deref().unwrap_or("Untitled"),
                    r.conversation.provider,
                    r.conversation.model.as_deref().unwrap_or("unknown"),
                    r.conversation.created_at.format("%Y-%m-%d"),
                    r.snippet.replace('[', "<mark>").replace(']', "</mark>")
                ))
                .collect::<Vec<_>>()
                .join("\n")
        }
    ))
}

/// Render conversation view
pub fn render_conversation(conversation: &Conversation, messages: &[Message]) -> anyhow::Result<String> {
    Ok(format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>{} - LLM Archive</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background: #f5f5f5;
        }}
        .header {{
            background: white;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .title {{
            font-size: 24px;
            font-weight: bold;
            margin-bottom: 10px;
        }}
        .meta {{
            color: #666;
            font-size: 14px;
        }}
        .messages {{
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            padding: 20px;
        }}
        .message {{
            margin-bottom: 20px;
            padding-bottom: 20px;
            border-bottom: 1px solid #eee;
        }}
        .message:last-child {{
            border-bottom: none;
        }}
        .message-role {{
            font-weight: bold;
            margin-bottom: 5px;
            text-transform: capitalize;
        }}
        .message-role.user {{
            color: #007bff;
        }}
        .message-role.assistant {{
            color: #28a745;
        }}
        .message-content {{
            white-space: pre-wrap;
            line-height: 1.6;
        }}
        .back-link {{
            display: inline-block;
            margin-bottom: 20px;
            color: #007bff;
            text-decoration: none;
        }}
        .back-link:hover {{
            text-decoration: underline;
        }}
        code {{
            background: #f4f4f4;
            padding: 2px 4px;
            border-radius: 3px;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 0.9em;
        }}
        pre {{
            background: #f4f4f4;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
        }}
        pre code {{
            background: none;
            padding: 0;
        }}
    </style>
</head>
<body>
    <a href="/" class="back-link">← Back to search</a>
    
    <div class="header">
        <div class="title">{}</div>
        <div class="meta">
            {} • {} • {} • {} messages
        </div>
    </div>
    
    <div class="messages">
        {}
    </div>
    
    <script>
        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {{
            if (e.key === 'Escape' || (e.key === 'Backspace' && !e.target.matches('input, textarea'))) {{
                e.preventDefault();
                window.history.back();
            }}
        }});
    </script>
</body>
</html>
    "#,
        conversation.title.as_deref().unwrap_or("Untitled"),
        conversation.title.as_deref().unwrap_or("Untitled"),
        conversation.provider,
        conversation.model.as_deref().unwrap_or("unknown"),
        conversation.created_at.format("%Y-%m-%d %H:%M"),
        messages.len(),
        messages.iter()
            .map(|msg| format!(
                r#"<div class="message">
                    <div class="message-role {}">{}</div>
                    <div class="message-content">{}</div>
                </div>"#,
                msg.role,
                msg.role,
                html_escape::encode_text(&msg.content)
            ))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}