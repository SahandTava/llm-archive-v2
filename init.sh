#!/bin/bash
# Initialize and run the LLM Archive V2

set -e

echo "🚀 LLM Archive V2 Setup"
echo "======================"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build the project
echo "📦 Building the project..."
cargo build --release

# Initialize the database
echo "🗄️ Initializing database..."
./target/release/llm-archive init

# Import existing data if available
if [ -d "../biji20LLMArchiv/exports" ]; then
    echo "📥 Found existing exports, importing data..."
    
    # Import ChatGPT conversations
    if [ -f "../biji20LLMArchiv/exports/conversations.json" ]; then
        echo "  - Importing ChatGPT conversations..."
        ./target/release/llm-archive import chatgpt ../biji20LLMArchiv/exports/conversations.json --python-bridge
    fi
    
    # Import Claude conversations
    if [ -d "../biji20LLMArchiv/exports/claude" ]; then
        echo "  - Importing Claude conversations..."
        ./target/release/llm-archive import claude ../biji20LLMArchiv/exports/claude --python-bridge
    fi
    
    # Import other providers...
    # Add more import commands as needed
fi

echo ""
echo "✅ Setup complete!"
echo ""
echo "To start the server, run:"
echo "  ./target/release/llm-archive serve"
echo ""
echo "Or use the development server:"
echo "  cargo run -- serve"