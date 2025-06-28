#!/usr/bin/env python3
"""
Parser for XAI/Grok export format
"""
import sys
import json
from pathlib import Path
from datetime import datetime

def parse_export(file_path):
    """Parse XAI/Grok export file and return normalized format"""
    conversations = []
    
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Grok export format analysis based on the file we found
    # The format is typically a single large JSON with conversation threads
    
    if isinstance(data, dict):
        # Check for different possible structures
        if 'conversations' in data:
            conv_list = data['conversations']
        elif 'threads' in data:
            conv_list = data['threads']
        elif 'data' in data:
            # Nested data structure
            inner_data = data['data']
            if isinstance(inner_data, list):
                conv_list = inner_data
            elif isinstance(inner_data, dict) and 'conversations' in inner_data:
                conv_list = inner_data['conversations']
            else:
                conv_list = [inner_data]
        else:
            # Assume single conversation
            conv_list = [data]
    else:
        conv_list = data
    
    for conv_data in conv_list:
        try:
            # Extract conversation metadata
            conversation = {
                'id': conv_data.get('id', conv_data.get('thread_id', conv_data.get('conversation_id', ''))),
                'title': conv_data.get('title', conv_data.get('subject', 'Untitled')),
                'created_at': conv_data.get('created_at', conv_data.get('timestamp', '')),
                'updated_at': conv_data.get('updated_at', conv_data.get('last_updated', '')),
                'model': 'grok-1',  # Default model
                'messages': []
            }
            
            # Parse timestamps (XAI might use Unix timestamps)
            for ts_field in ['created_at', 'updated_at']:
                ts_value = conversation[ts_field]
                if ts_value and isinstance(ts_value, (int, float)):
                    # Unix timestamp
                    conversation[ts_field] = datetime.fromtimestamp(ts_value).isoformat()
                elif not ts_value:
                    conversation[ts_field] = datetime.now().isoformat()
            
            # Extract user info if available
            if 'user' in conv_data:
                conversation['user_id'] = conv_data['user'].get('id') if isinstance(conv_data['user'], dict) else str(conv_data['user'])
            
            # Parse messages
            messages_data = []
            if 'messages' in conv_data:
                messages_data = conv_data['messages']
            elif 'exchanges' in conv_data:
                messages_data = conv_data['exchanges']
            elif 'turns' in conv_data:
                messages_data = conv_data['turns']
            
            for i, msg_data in enumerate(messages_data):
                # XAI/Grok message format
                role = msg_data.get('role', msg_data.get('sender', msg_data.get('type', '')))
                
                # Normalize role
                if role.lower() in ['user', 'human', 'question']:
                    role = 'user'
                elif role.lower() in ['grok', 'assistant', 'ai', 'model', 'answer']:
                    role = 'assistant'
                elif role.lower() == 'system':
                    role = 'system'
                
                message = {
                    'id': msg_data.get('id', msg_data.get('message_id', f'msg_{i}')),
                    'role': role,
                    'content': msg_data.get('content', msg_data.get('text', msg_data.get('message', ''))),
                    'created_at': msg_data.get('created_at', msg_data.get('timestamp', conversation['created_at'])),
                }
                
                # Handle timestamp conversion
                if isinstance(message['created_at'], (int, float)):
                    message['created_at'] = datetime.fromtimestamp(message['created_at']).isoformat()
                
                # Extract model info if per-message
                if 'model' in msg_data:
                    message['model'] = msg_data['model']
                elif 'engine' in msg_data:
                    message['model'] = msg_data['engine']
                
                # Extract token counts if available
                if 'token_count' in msg_data:
                    message['tokens'] = msg_data['token_count']
                elif 'tokens' in msg_data:
                    message['tokens'] = msg_data['tokens']
                
                # Handle attachments/references
                if 'attachments' in msg_data:
                    message['attachments'] = msg_data['attachments']
                elif 'references' in msg_data:
                    message['attachments'] = msg_data['references']
                
                conversation['messages'].append(message)
            
            # Try to extract model/settings from conversation metadata
            if 'model' in conv_data:
                conversation['model'] = conv_data['model']
            elif 'settings' in conv_data and 'model' in conv_data['settings']:
                conversation['model'] = conv_data['settings']['model']
            
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