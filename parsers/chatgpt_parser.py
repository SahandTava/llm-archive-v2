#!/usr/bin/env python3
"""
Wrapper for V1 ChatGPT parser to work with PyO3 bridge
"""
import sys
import json
from pathlib import Path
from datetime import datetime

# Add V1 project to path
sys.path.insert(0, '/home/bijan/LLMArchGH/biji20LLMArchiv')

try:
    from llm_archive.providers.chatgpt import ChatGPTProvider
    V1_AVAILABLE = True
except ImportError:
    V1_AVAILABLE = False

def parse_export(file_path):
    """Parse ChatGPT export file and return normalized format"""
    conversations = []
    
    if V1_AVAILABLE:
        # Use V1 parser
        provider = ChatGPTProvider(None)  # No DB needed for parsing
        # The V1 parser expects a different interface, so we'll parse directly
    
    # Fall back to direct parsing
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    for conv_data in data:
        try:
            conversation = {
                'id': conv_data.get('conversation_id', conv_data.get('id')),
                'title': conv_data.get('title', 'Untitled'),
                'created_at': datetime.fromtimestamp(conv_data.get('create_time', 0)).isoformat(),
                'updated_at': datetime.fromtimestamp(conv_data.get('update_time', conv_data.get('create_time', 0))).isoformat(),
                'model': conv_data.get('default_model_slug'),
                'messages': []
            }
            
            # Extract metadata
            if 'gizmo_id' in conv_data:
                conversation['gizmo_id'] = conv_data['gizmo_id']
            
            # Parse messages from mapping
            mapping = conv_data.get('mapping', {})
            messages_by_id = {}
            
            # First pass: create all messages
            for node_id, node in mapping.items():
                if node.get('message'):
                    msg = node['message']
                    author = msg.get('author', {})
                    
                    message = {
                        'id': msg.get('id', node_id),
                        'role': author.get('role', 'unknown'),
                        'content': '',
                        'created_at': datetime.fromtimestamp(msg.get('create_time', 0)).isoformat() if msg.get('create_time') else conversation['created_at'],
                        'parent': node.get('parent'),
                        'model': msg.get('metadata', {}).get('model_slug')
                    }
                    
                    # Extract content
                    content_obj = msg.get('content', {})
                    if content_obj.get('content_type') == 'text':
                        parts = content_obj.get('parts', [])
                        if parts:
                            message['content'] = '\n'.join(str(p) for p in parts if p)
                    elif 'text' in content_obj:
                        message['content'] = content_obj['text']
                    
                    # Extract metadata
                    metadata = msg.get('metadata', {})
                    if metadata:
                        if 'finish_details' in metadata:
                            message['finish_reason'] = metadata['finish_details'].get('type')
                        if 'model_slug' in metadata:
                            message['model'] = metadata['model_slug']
                    
                    messages_by_id[node_id] = message
            
            # Second pass: build conversation flow
            def get_message_chain(node_id, visited=None):
                if visited is None:
                    visited = set()
                if node_id in visited or node_id not in messages_by_id:
                    return []
                visited.add(node_id)
                
                message = messages_by_id[node_id]
                chain = []
                
                # Add parent messages first
                if message['parent'] and message['parent'] in messages_by_id:
                    chain.extend(get_message_chain(message['parent'], visited))
                
                # Add this message
                chain.append(message)
                
                return chain
            
            # Find leaf nodes (messages with no children)
            all_children = set()
            for node_id, node in mapping.items():
                for child in node.get('children', []):
                    all_children.add(child)
            
            leaf_nodes = [node_id for node_id in mapping.keys() if node_id not in all_children and node_id in messages_by_id]
            
            # If no leaf nodes found, try to find the current_node
            if not leaf_nodes and 'current_node' in conv_data:
                leaf_nodes = [conv_data['current_node']]
            
            # Get the longest conversation chain
            longest_chain = []
            for leaf in leaf_nodes:
                chain = get_message_chain(leaf)
                if len(chain) > len(longest_chain):
                    longest_chain = chain
            
            # Remove parent field and add messages to conversation
            for msg in longest_chain:
                msg.pop('parent', None)
                conversation['messages'].append(msg)
            
            conversations.append(conversation)
            
        except Exception as e:
            print(f"Error parsing conversation: {e}")
            continue
    
    return conversations

if __name__ == '__main__':
    # Test the parser
    if len(sys.argv) > 1:
        result = parse_export(sys.argv[1])
        print(f"Parsed {len(result)} conversations")
        if result:
            print(f"First conversation has {len(result[0]['messages'])} messages")