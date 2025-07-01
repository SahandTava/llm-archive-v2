#!/usr/bin/env python3
"""
Wrapper for V1 Claude parser to work with PyO3 bridge
"""
import sys
import json
from pathlib import Path
from datetime import datetime
# import dateutil.parser

# Add V1 project to path
sys.path.insert(0, '/home/bijan/LLMArchGH/biji20LLMArchiv')

try:
    from llm_archive.providers.claude import ClaudeProvider
    V1_AVAILABLE = True
except ImportError:
    V1_AVAILABLE = False

def parse_export(file_path):
    """Parse Claude export file and return normalized format"""
    conversations = []
    
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Handle both single conversation and array formats
    if isinstance(data, dict):
        data = [data]
    
    for conv_data in data:
        try:
            conversation = {
                'id': conv_data.get('uuid', conv_data.get('id')),
                'title': conv_data.get('name', 'Untitled'),
                'created_at': conv_data.get('created_at', ''),
                'updated_at': conv_data.get('updated_at', conv_data.get('created_at', '')),
                'messages': []
            }
            
            # Parse timestamps - Claude uses ISO format already
            if not conversation['created_at']:
                conversation['created_at'] = datetime.now().isoformat()
            
            if not conversation['updated_at']:
                conversation['updated_at'] = conversation['created_at']
            
            # Extract account/project info
            if 'account' in conv_data:
                conversation['user_id'] = conv_data['account'].get('uuid') if isinstance(conv_data['account'], dict) else str(conv_data['account'])
            
            # Parse messages
            for msg_data in conv_data.get('chat_messages', []):
                message = {
                    'id': msg_data.get('uuid', ''),
                    'role': 'user' if msg_data.get('sender') == 'human' else 'assistant',
                    'content': msg_data.get('text', ''),
                    'created_at': msg_data.get('created_at', ''),
                }
                
                # Parse message timestamp - Claude uses ISO format
                if not message['created_at']:
                    message['created_at'] = conversation['created_at']
                
                # Handle attachments
                files = msg_data.get('files', [])
                if files:
                    attachments = []
                    for file_data in files:
                        attachment = {
                            'file_name': file_data.get('file_name', ''),
                            'file_type': file_data.get('file_type', ''),
                            'file_size': file_data.get('file_size'),
                        }
                        if 'extracted_content' in file_data and file_data['extracted_content']:
                            # Add extracted content to message
                            message['content'] += f"\n\n[Attachment: {attachment['file_name']}]\n{file_data['extracted_content']}"
                        attachments.append(attachment)
                    
                    message['attachments'] = attachments
                
                # Check if message was edited
                if msg_data.get('edited'):
                    message['edited'] = True
                
                conversation['messages'].append(message)
            
            # Try to infer model from conversation
            # Claude doesn't always include model info in exports
            if 'model' in conv_data:
                conversation['model'] = conv_data['model']
            elif 'settings' in conv_data and 'model' in conv_data['settings']:
                conversation['model'] = conv_data['settings']['model']
            
            # Extract other settings
            if 'settings' in conv_data:
                settings = conv_data['settings']
                if 'temperature' in settings:
                    conversation['temperature'] = settings['temperature']
                if 'max_tokens' in settings:
                    conversation['max_tokens'] = settings['max_tokens']
                if 'system_prompt' in settings:
                    conversation['system_prompt'] = settings['system_prompt']
            
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