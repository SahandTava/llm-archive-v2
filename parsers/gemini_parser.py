#!/usr/bin/env python3
"""
Parser for Gemini/Google AI Studio export format
"""
import sys
import json
from pathlib import Path
from datetime import datetime

def parse_export(file_path):
    """Parse Gemini export file and return normalized format"""
    conversations = []
    
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Gemini format varies - handle different structures
    if isinstance(data, dict):
        # Single conversation or wrapped format
        if 'conversations' in data:
            conv_list = data['conversations']
        elif 'messages' in data:
            # Direct messages format
            conv_list = [data]
        else:
            conv_list = [data]
    else:
        conv_list = data
    
    for conv_data in conv_list:
        try:
            conversation = {
                'id': conv_data.get('id', conv_data.get('conversation_id', '')),
                'title': conv_data.get('title', conv_data.get('name', 'Untitled')),
                'created_at': conv_data.get('created_at', ''),
                'updated_at': conv_data.get('updated_at', ''),
                'model': conv_data.get('model', 'gemini-pro'),
                'messages': []
            }
            
            # Parse timestamps
            if not conversation['created_at']:
                conversation['created_at'] = datetime.now().isoformat()
            if not conversation['updated_at']:
                conversation['updated_at'] = conversation['created_at']
            
            # Extract settings/metadata
            if 'settings' in conv_data:
                settings = conv_data['settings']
                if 'temperature' in settings:
                    conversation['temperature'] = settings['temperature']
                if 'max_output_tokens' in settings:
                    conversation['max_tokens'] = settings['max_output_tokens']
                if 'system_instruction' in settings:
                    conversation['system_prompt'] = settings['system_instruction']
            
            # Parse messages
            messages_data = conv_data.get('messages', conv_data.get('turns', []))
            
            for i, msg_data in enumerate(messages_data):
                # Gemini uses different field names
                if isinstance(msg_data, dict):
                    role = msg_data.get('role', msg_data.get('author', ''))
                    content = msg_data.get('content', msg_data.get('text', ''))
                    
                    # Normalize role names
                    if role.lower() in ['user', 'human']:
                        role = 'user'
                    elif role.lower() in ['model', 'assistant', 'gemini']:
                        role = 'assistant'
                    
                    message = {
                        'id': msg_data.get('id', f'msg_{i}'),
                        'role': role,
                        'content': content,
                        'created_at': msg_data.get('created_at', conversation['created_at']),
                    }
                    
                    # Extract parts if present (multimodal content)
                    if 'parts' in msg_data:
                        parts = msg_data['parts']
                        text_parts = []
                        for part in parts:
                            if isinstance(part, str):
                                text_parts.append(part)
                            elif isinstance(part, dict):
                                if 'text' in part:
                                    text_parts.append(part['text'])
                                elif 'inline_data' in part:
                                    # Handle images/files
                                    mime_type = part['inline_data'].get('mime_type', 'unknown')
                                    text_parts.append(f"[Attached: {mime_type}]")
                        
                        if text_parts:
                            message['content'] = '\n'.join(text_parts)
                    
                    # Safety ratings
                    if 'safety_ratings' in msg_data:
                        message['safety_ratings'] = msg_data['safety_ratings']
                    
                    conversation['messages'].append(message)
            
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