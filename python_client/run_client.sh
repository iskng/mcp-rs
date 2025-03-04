#!/bin/bash
# Script to set up Python environment and run the MCP client

set -e

# Create a virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
fi

# Activate the virtual environment
echo "Activating virtual environment..."
source venv/bin/activate

# Install dependencies
echo "Installing dependencies..."
pip install --upgrade pip
pip install -r requirements.txt

# Default parameters
SERVER_URL="http://127.0.0.1:8090/sse"
DEBUG="--debug"  # Enable debug by default for troubleshooting

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-debug)
            DEBUG=""
            shift
            ;;
        --server)
            SERVER_URL="$2"
            shift 2
            ;;
        *)
            # Pass any other arguments to the Python script
            break
            ;;
    esac
done

# Allow environment variable overrides
if [ -n "$MCP_SERVER_URL" ]; then
    SERVER_URL="$MCP_SERVER_URL"
fi

# Simple check if the server is running 
echo "Checking if server is running..."
SERVER_HOST=$(echo $SERVER_URL | sed -E 's|^https?://||' | sed -E 's|/.*$||' | sed -E 's|:.*$||')
SERVER_PORT=$(echo $SERVER_URL | sed -E 's|^https?://[^:]+:([0-9]+).*$|\1|')
if [ "$SERVER_PORT" = "$SERVER_URL" ]; then
    # Default port if not specified
    SERVER_PORT=80
    if [[ $SERVER_URL == https://* ]]; then
        SERVER_PORT=443
    fi
fi

if command -v nc &> /dev/null; then
    if nc -z -w1 "$SERVER_HOST" "$SERVER_PORT"; then
        echo "Server appears to be running at $SERVER_HOST:$SERVER_PORT"
    else
        echo "WARNING: Cannot connect to $SERVER_HOST:$SERVER_PORT, server may not be running!"
    fi
fi

# Run the client
echo "==================== MCP CALCULATOR TOOL TEST ===================="
echo "This test will:"
echo "1. Connect to the MCP server at $SERVER_URL"
echo "2. Initialize the connection"
echo "3. List available tools"
echo "4. Test the calculator tool with multiple operations"
echo "   - add(5, 3)"
echo "   - subtract(10, 4)" 
echo "   - multiply(6, 7)"
echo "   - divide(15, 3)"
echo "==============================================================="
echo ""
echo "Running MCP client..."

python client.py \
    --server "$SERVER_URL" \
    $DEBUG \
    "$@" 