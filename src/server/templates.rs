use askama::Template;
use crate::models::{Conversation, Message, SearchResult};
use super::Stats;

/// Index page template
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    pub stats: &'a Stats,
}

/// Search results template
#[derive(Template)]
#[template(path = "search.html")]
pub struct SearchTemplate<'a> {
    pub query: &'a str,
    pub results: &'a [SearchResult],
}

/// Conversation view template
#[derive(Template)]
#[template(path = "conversation.html")]
pub struct ConversationTemplate<'a> {
    pub conversation: &'a Conversation,
    pub messages: &'a [Message],
}

/// Render index page
pub fn render_index(stats: &Stats) -> anyhow::Result<String> {
    let template = IndexTemplate { stats };
    Ok(template.render()?)
}

/// Render search results
pub fn render_search_results(query: &str, results: &[SearchResult]) -> anyhow::Result<String> {
    let template = SearchTemplate { query, results };
    Ok(template.render()?)
}

/// Render conversation view
pub fn render_conversation(conversation: &Conversation, messages: &[Message]) -> anyhow::Result<String> {
    let template = ConversationTemplate { conversation, messages };
    Ok(template.render()?)
}