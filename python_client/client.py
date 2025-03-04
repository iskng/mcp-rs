#!/usr/bin/env python3
"""
MCP Python Client using SSE Transport

This script demonstrates how to connect to an MCP server using the official MCP Python library
with the Server-Sent Events (SSE) transport. It handles the initial connection and handshake.
"""

import os
import sys
import json
import asyncio
import logging
import argparse
import traceback
from contextlib import AsyncExitStack

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    datefmt="%H:%M:%S"
)
log = logging.getLogger("mcp-client")

class McpClient:
    """Client for interacting with MCP servers using the official MCP library."""
    
    def __init__(self, server_url, debug=False):
        self.server_url = server_url
        if debug:
            log.setLevel(logging.DEBUG)
            self.debug_mode = True
        else:
            self.debug_mode = False
        
        # Ensure server URL is properly formatted for SSE
        # if not self.server_url.endswith('/'):
        #     self.server_url += '/'
        
        # Setup exit stack for resource management
        self.exit_stack = AsyncExitStack()
        self.session = None
    
    async def connect(self):
        """Connect to the MCP server using the SSE transport."""
        try:
            # Import MCP library
            log.info("Importing MCP library...")
            from mcp import ClientSession
            from mcp.client.sse import sse_client
            
            log.info(f"Connecting to MCP server at {self.server_url}")
            
            # Connect using SSE transport
            log.info("Establishing SSE connection...")
            transport_streams = await self.exit_stack.enter_async_context(sse_client(self.server_url))
            
            # Create client session with transport
            self.session = await self.exit_stack.enter_async_context(
                ClientSession(
                    read_stream=transport_streams[0],
                    write_stream=transport_streams[1]
                )
            )
            
            
            return True
        except ImportError as e:
            log.error(f"Failed to import MCP library: {e}")
            if self.debug_mode:
                log.debug("Make sure the MCP library is installed: pip install mcp")
            return False
        except Exception as e:
            log.error(f"Error connecting to MCP server: {e}")
            if self.debug_mode:
                traceback.print_exc()
            return False
    
    async def initialize(self):
        """Initialize the connection to the MCP server."""
        if not self.session:
            log.error("Cannot initialize - no active session")
            return False
        
        try:
            log.info("Initializing MCP session...")
            # The initialize method handles the handshake automatically
            await self.session.initialize()
            log.info("Session initialized successfully!")
            return True
        except Exception as e:
            log.error(f"Error initializing session: {e}")
            if self.debug_mode:
                traceback.print_exc()
            return False
    
    async def list_tools(self):
        """List available tools from the server."""
        if not self.session:
            log.error("Cannot list tools - no active session")
            return None
        
        try:
            log.info("Listing available tools...")
            tools = await self.session.list_tools()
            return tools
        except Exception as e:
            log.error(f"Error listing tools: {e}")
            return None
    
    async def call_calculator(self, a, b, operation="add"):
        """Call the calculator tool as a test."""
        if not self.session:
            log.error("Cannot call calculator - no active session")
            return None
        
        try:
            log.info(f"Calling calculator tool: {operation}({a}, {b})...")
            result = await self.session.call_tool(
                "calculator", 
                {
                    "operation": operation,
                    "a": a,
                    "b": b
                }
            )
            
            # Log the result in a readable format
            try:
                if hasattr(result, "model_dump"):
                    log_result = json.dumps(result.model_dump(), indent=2)
                elif hasattr(result, "dict"):
                    log_result = json.dumps(result.dict(), indent=2)
                else:
                    log_result = str(result)
                log.info(f"Calculator result: {log_result}")
            except Exception as e:
                log.error(f"Error formatting result: {e}")
                log.info(f"Raw calculator result: {result}")
            
            return result
        except Exception as e:
            log.error(f"Error calling calculator: {e}")
            if self.debug_mode:
                traceback.print_exc()
            return None
    
    async def receive_messages(self):
        """Listen for messages from the server."""
        if not self.session:
            log.error("Cannot receive messages - no active session")
            return
        
        try:
            log.info("Starting message listener...")
            async for message in self.session.incoming_messages:
                if isinstance(message, Exception):
                    log.error(f"Error in message stream: {message}")
                    continue
                
                log.info(f"Received message: {message}")
        except Exception as e:
            log.error(f"Error in message listener: {e}")
            if self.debug_mode:
                traceback.print_exc()
    
    async def cleanup(self):
        """Clean up resources."""
        try:
            log.info("Cleaning up resources...")
            await self.exit_stack.aclose()
            self.session = None
        except Exception as e:
            log.error(f"Error during cleanup: {e}")
    
    async def run(self):
        """Run the client: connect, initialize, and interact with the server."""
        log.info(f"Starting MCP client")
        log.info(f"Python version: {sys.version}")
        
        try:
            # Connect to the server
            if not await self.connect():
                log.error("Failed to connect to server")
                return
            
            # Initialize the connection
            if not await self.initialize():
                log.error("Failed to initialize connection")
                return
            
            # Create a task for receiving messages
            receive_task = asyncio.create_task(self.receive_messages())
            
            # List tools
            tools_result = await self.list_tools()
            if tools_result:
                # Convert the Pydantic model to a serializable dict
                try:
                    # Try modern Pydantic v2 method first
                    if hasattr(tools_result, "model_dump"):
                        tools_dict = tools_result.model_dump()
                    # Fall back to Pydantic v1 method
                    elif hasattr(tools_result, "dict"):
                        tools_dict = tools_result.dict()
                    else:
                        # Manual conversion as last resort
                        tools_dict = {"tools": []}
                        for tool in tools_result.tools:
                            if hasattr(tool, "model_dump"):
                                tools_dict["tools"].append(tool.model_dump())
                            elif hasattr(tool, "dict"):
                                tools_dict["tools"].append(tool.dict())
                            else:
                                tools_dict["tools"].append({
                                    "name": tool.name,
                                    "description": tool.description,
                                    "inputSchema": getattr(tool, "inputSchema", {})
                                })
                    
                    log.info(f"Available tools: {json.dumps(tools_dict, indent=2)}")
                except Exception as e:
                    log.error(f"Error serializing tools result: {e}")
                    if self.debug_mode:
                        traceback.print_exc()
                
                # Check if the calculator tool is available
                calculator_tool = None
                for tool in tools_result.tools:
                    if tool.name == "calculator":
                        calculator_tool = tool
                        break
                
                if calculator_tool:
                    log.info(f"Found calculator tool: {calculator_tool.description}")
                    
                    # Get the input schema from the tool
                    input_schema = {}
                    if hasattr(calculator_tool, "inputSchema"):
                        input_schema = calculator_tool.inputSchema
                    
                    log.info("Input schema: " + json.dumps(input_schema, indent=2))
                    
                    # Test the calculator with different operations
                    operations = [
                        {"operation": "add", "a": 5, "b": 3},
                        {"operation": "subtract", "a": 10, "b": 4},
                        {"operation": "multiply", "a": 6, "b": 7},
                        {"operation": "divide", "a": 15, "b": 3}
                    ]
                    
                    for params in operations:
                        try:
                            result = await self.call_calculator(
                                params["a"], 
                                params["b"], 
                                params["operation"]
                            )
                            if result:
                                op = params["operation"]
                                a = params["a"]
                                b = params["b"]
                                # Handle result as a Pydantic model
                                if hasattr(result, "result"):
                                    # Direct attribute access
                                    calc_result = result.result
                                elif hasattr(result, "model_dump"):
                                    # Pydantic v2
                                    result_dict = result.model_dump()
                                    calc_result = result_dict.get("result", {})
                                elif hasattr(result, "dict"):
                                    # Pydantic v1
                                    result_dict = result.dict()
                                    calc_result = result_dict.get("result", {})
                                else:
                                    # Fallback for unknown format
                                    calc_result = str(result)
                                log.info(f"✅ {op}({a}, {b}) = {calc_result}")
                        except Exception as e:
                            log.error(f"❌ Error calling calculator with {params}: {e}")
                            if self.debug_mode:
                                traceback.print_exc()
                else:
                    log.warning("Calculator tool not found in available tools")
            else:
                log.error("Failed to retrieve tools list")
            
            # Keep the connection open for a while to receive messages
            log.info("Keeping connection open for 10 seconds...")
            try:
                await asyncio.wait_for(receive_task, timeout=10)
            except asyncio.TimeoutError:
                log.info("Connection time limit reached")
            
        except KeyboardInterrupt:
            log.info("Client interrupted by user")
        except Exception as e:
            log.error(f"Unexpected error: {e}")
            if self.debug_mode:
                traceback.print_exc()
        finally:
            await self.cleanup()

async def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description='MCP SSE Client')
    parser.add_argument('--server', default='http://127.0.0.1:8090/sse',
                       help='MCP server URL (default: http://127.0.0.1:8090)')
    parser.add_argument('--debug', action='store_true',
                       help='Enable debug mode with verbose output')
    args = parser.parse_args()
    
    client = McpClient(args.server, args.debug)
    await client.run()

if __name__ == "__main__":
    asyncio.run(main()) 