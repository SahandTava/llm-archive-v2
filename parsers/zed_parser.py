#!/usr/bin/env python3
"""
Parser for Zed AI export format
"""
import sys
import json
from pathlib import Path
from datetime import datetime

def parse_export(file_path):
    """Parse Zed AI export file and return normalized format"""
    conversations = []
    
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Zed AI assistant format (from code editor)
    # Usually exports as workspace conversations
    
    if isinstance(data, dict):
        if 'conversations' in data:
            conv_list = data['conversations']
        elif 'sessions' in data:
            conv_list = data['sessions']
        elif 'workspace_conversations' in data:
            conv_list = data['workspace_conversations']
        else:
            conv_list = [data]
    else:
        conv_list = data
    
    for conv_data in conv_list:
        try:
            conversation = {
                'id': conv_data.get('id', conv_data.get('session_id', '')),
                'title': conv_data.get('title', conv_data.get('file_path', 'Zed AI Session')),
                'created_at': conv_data.get('created_at', conv_data.get('started_at', '')),
                'updated_at': conv_data.get('updated_at', conv_data.get('ended_at', '')),
                'model': conv_data.get('model', 'zed-ai'),
                'messages': []
            }
            
            # Parse timestamps
            if not conversation['created_at']:
                conversation['created_at'] = datetime.now().isoformat()
            if not conversation['updated_at']:
                conversation['updated_at'] = conversation['created_at']
            
            # Extract workspace/project context
            if 'workspace' in conv_data:
                conversation['workspace'] = conv_data['workspace']
            if 'file_path' in conv_data:
                conversation['file_path'] = conv_data['file_path']
            if 'language' in conv_data:
                conversation['language'] = conv_data['language']
            
            # Parse messages
            messages_data = conv_data.get('messages', conv_data.get('interactions', []))
            
            for i, msg_data in enumerate(messages_data):
                role = msg_data.get('role', msg_data.get('type', ''))
                
                # Normalize Zed-specific roles
                if role.lower() in ['user', 'human', 'developer']:
                    role = 'user'
                elif role.lower() in ['assistant', 'ai', 'zed']:
                    role = 'assistant'
                elif role.lower() == 'system':
                    role = 'system'
                
                content = msg_data.get('content', msg_data.get('text', ''))
                
                # Handle code blocks and context
                if 'code' in msg_data:
                    code = msg_data['code']
                    language = msg_data.get('language', 'text')
                    content = f"{content}\n\n```{language}\n{code}\n```"
                
                if 'context' in msg_data:
                    # Add file context
                    context = msg_data['context']
                    if isinstance(context, dict):
                        if 'file' in context:
                            content = f"[File: {context['file']}]\n{content}"
                        if 'selection' in context:
                            content = f"[Selection: lines {context['selection']['start']}-{context['selection']['end']}]\n{content}"
                
                message = {
                    'id': msg_data.get('id', f'msg_{i}'),
                    'role': role,
                    'content': content,
                    'created_at': msg_data.get('created_at', msg_data.get('timestamp', conversation['created_at'])),
                }
                
                # Extract additional metadata
                if 'language' in msg_data:
                    message['language'] = msg_data['language']
                
                if 'diagnostics' in msg_data:
                    # Code diagnostics/errors
                    message['diagnostics'] = msg_data['diagnostics']
                
                if 'suggestions' in msg_data:
                    message['suggestions'] = msg_data['suggestions']
                
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