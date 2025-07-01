#!/bin/bash
# Test import functionality with real export files

set -e

echo "🧪 Testing LLM Archive V2 Import Functionality"
echo "============================================="

# Test data paths
CHATGPT_SMALL="/home/bijan/LLMArchGH/data/Archiv4LLMArchive/endeJuni/f1dd891a54c23f2972ae1e69f9d89d8f48eab577570ee6da587a17ebe8729fd6-2025-06-27-08-13-03-c96fe9ce0eb04906b5ae7d89da04835b/conversations.json"
CHATGPT_LARGE="/home/bijan/LLMArchGH/data/Archiv4LLMArchive/b11fcf69015b52d6b411a219175298a35f91be825d76ed47bb7a540bd88363e2-2025-04-22-01-13-32-d63f53d992cd4a78a5c474bc1eb022cb/conversations.json"
CLAUDE_DATA="/home/bijan/LLMArchGH/data/Archiv4LLMArchive/endeJuni/data-2025-06-27-08-12-19/conversations.json"
GROK_DATA="/home/bijan/LLMArchGH/data/Archiv4LLMArchive/endeJuni/ttl/30d/export_data/72a4171a-0639-41e7-8187-14e7147967b9/prod-grok-backend.json"

# Create test database
TEST_DB="./test_import.db"
rm -f $TEST_DB

echo "📊 File sizes:"
echo "  ChatGPT (small): $(du -h "$CHATGPT_SMALL" | cut -f1)"
echo "  ChatGPT (large): $(du -h "$CHATGPT_LARGE" | cut -f1)"
echo "  Claude: $(du -h "$CLAUDE_DATA" | cut -f1)"
echo "  Grok: $(du -h "$GROK_DATA" | cut -f1)"
echo ""

# Test Python bridge connectivity first
echo "🐍 Testing Python bridge..."
python3 -c "
import sys
sys.path.append('/home/bijan/LLMArchGH/biji20LLMArchiv/parsers')
try:
    import chatgpt_parser
    print('  ✓ ChatGPT parser available')
except ImportError as e:
    print(f'  ✗ ChatGPT parser not found: {e}')

try:
    import claude_parser
    print('  ✓ Claude parser available')
except ImportError as e:
    print(f'  ✗ Claude parser not found: {e}')
"

echo ""
echo "📥 Testing imports (this will use sample data for now)..."

# Since we can't run Rust directly, let's create a Python test script
cat > test_parser.py << 'EOF'
import json
import sys
from datetime import datetime

def test_chatgpt_format(file_path):
    """Test parsing ChatGPT export format"""
    try:
        with open(file_path, 'r') as f:
            data = json.load(f)
        
        print(f"✓ Successfully loaded ChatGPT file")
        print(f"  - Total conversations: {len(data)}")
        
        # Sample first conversation
        if data:
            first = data[0]
            print(f"  - First conversation ID: {first.get('id', 'N/A')}")
            print(f"  - Title: {first.get('title', 'N/A')[:50]}...")
            print(f"  - Messages in mapping: {len(first.get('mapping', {}))}")
        
        return True
    except Exception as e:
        print(f"✗ Failed to parse ChatGPT file: {e}")
        return False

def test_claude_format(file_path):
    """Test parsing Claude export format"""
    try:
        with open(file_path, 'r') as f:
            data = json.load(f)
        
        print(f"✓ Successfully loaded Claude file")
        
        # Claude format can be either array or single conversation
        if isinstance(data, list):
            print(f"  - Total conversations: {len(data)}")
            if data:
                first = data[0]
                print(f"  - First conversation ID: {first.get('uuid', 'N/A')}")
                print(f"  - Messages: {len(first.get('chat_messages', []))}")
        else:
            print(f"  - Single conversation format")
            print(f"  - ID: {data.get('uuid', 'N/A')}")
            print(f"  - Messages: {len(data.get('chat_messages', []))}")
        
        return True
    except Exception as e:
        print(f"✗ Failed to parse Claude file: {e}")
        return False

def test_file_structure(file_path, provider):
    """Test file structure and extract metadata"""
    try:
        with open(file_path, 'r') as f:
            data = json.load(f)
        
        print(f"\n📄 {provider} file structure analysis:")
        
        # Get sample of keys
        if isinstance(data, list) and data:
            sample = data[0]
        elif isinstance(data, dict):
            sample = data
        else:
            print("  - Unknown structure")
            return
        
        print(f"  - Root type: {type(data).__name__}")
        print(f"  - Top-level keys: {list(sample.keys())[:10]}")
        
        # Look for timestamp fields
        timestamp_fields = [k for k in sample.keys() if 'time' in k.lower() or 'date' in k.lower() or 'created' in k.lower()]
        if timestamp_fields:
            print(f"  - Timestamp fields: {timestamp_fields}")
        
        # Look for model information
        model_fields = [k for k in sample.keys() if 'model' in k.lower()]
        if model_fields:
            print(f"  - Model fields: {model_fields}")
            
    except Exception as e:
        print(f"  - Error analyzing structure: {e}")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        file_path = sys.argv[1]
        provider = sys.argv[2] if len(sys.argv) > 2 else "Unknown"
        
        if provider.lower() == "chatgpt":
            test_chatgpt_format(file_path)
        elif provider.lower() == "claude":
            test_claude_format(file_path)
        
        test_file_structure(file_path, provider)
EOF

# Test each format
echo -e "\n1️⃣ Testing ChatGPT format (small file):"
python3 test_parser.py "$CHATGPT_SMALL" "ChatGPT"

echo -e "\n2️⃣ Testing Claude format:"
python3 test_parser.py "$CLAUDE_DATA" "Claude"

# Clean up
rm -f test_parser.py

echo -e "\n✅ Test analysis complete!"
echo -e "\nNext steps:"
echo "  1. Build the Rust project: cargo build --release"
echo "  2. Initialize database: ./target/release/llm-archive init --database $TEST_DB"
echo "  3. Import ChatGPT: ./target/release/llm-archive import chatgpt \"$CHATGPT_SMALL\" --database $TEST_DB --python-bridge"
echo "  4. Import Claude: ./target/release/llm-archive import claude \"$CLAUDE_DATA\" --database $TEST_DB --python-bridge"
echo "  5. Test search: ./target/release/llm-archive search \"test query\" --database $TEST_DB"