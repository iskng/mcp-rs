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
    
    async def list_resources(self):
        """List available resources from the server."""
        if not self.session:
            log.error("Cannot list resources - no active session")
            return None
        
        try:
            log.info("Listing available resources...")
            params = {}  # Can include mime_type filter if needed
            resources = await self.session.list_resources()
            return resources
        except Exception as e:
            log.error(f"Error listing resources: {e}")
            if self.debug_mode:
                traceback.print_exc()
            return None
    
    async def read_resource(self, uri):
        """Read a resource by URI."""
        if not self.session:
            log.error("Cannot read resource - no active session")
            return None
        
        try:
            log.info(f"Reading resource: {uri}")
            resource = await self.session.read_resource(uri)
            return resource
        except Exception as e:
            log.error(f"Error reading resource: {e}")
            if self.debug_mode:
                traceback.print_exc()
            return None
    
    async def call_calculator(self, a, b, operation="add"):
        """Call the calculator tool as a test."""
        if not self.session:
            log.error("Cannot call calculator - no active session")
            return None
        
        try:
            
            # Use the operation as provided (from the schema)
            # No need to modify the case since we're now using values directly from the schema
            log.info(f"Calling calculator tool: {operation}({a}, {b})...")
            result = await self.session.call_tool(
                "Calculator", 
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
            
            # List resources
            log.info("--- Listing Available Resources ---")
            resources_result = await self.list_resources()
            if resources_result:
                try:
                    # Process resources with custom serialization to handle AnyUrl
                    resources_list = []
                    if hasattr(resources_result, "resources"):
                        for resource in resources_result.resources:
                            # Handle AnyUrl by converting to string
                            resource_dict = {}
                            resource_dict["uri"] = str(resource.uri) if hasattr(resource, "uri") else "unknown"
                            resource_dict["name"] = resource.name if hasattr(resource, "name") else "unknown"
                            
                            if hasattr(resource, "description") and resource.description is not None:
                                resource_dict["description"] = resource.description
                            
                            if hasattr(resource, "mime_type") and resource.mime_type is not None:
                                resource_dict["mime_type"] = resource.mime_type
                            
                            resources_list.append(resource_dict)
                    
                    resources_dict = {"resources": resources_list}
                    log.info(f"Available resources: {json.dumps(resources_dict, indent=2)}")
                    
                    # Read each resource
                    if resources_list:
                        for resource in resources_list:
                            uri = resource.get("uri")
                            if uri:
                                log.info(f"--- Reading Resource: {uri} ---")
                                resource_content = await self.read_resource(uri)
                                
                                if resource_content:
                                    # Handle different content types
                                    content_dict = {}
                                    
                                    # Handle text content
                                    if hasattr(resource_content, "text"):
                                        content_dict["content_type"] = "text"
                                        content_dict["content"] = resource_content.text
                                        content_dict["mime_type"] = resource_content.mime_type
                                    # Handle binary content
                                    elif hasattr(resource_content, "blob"):
                                        content_dict["content_type"] = "binary"
                                        content_dict["content"] = "<binary data>"
                                        content_dict["mime_type"] = resource_content.mime_type
                                    # Fall back to generic serialization
                                    else:
                                        if hasattr(resource_content, "model_dump"):
                                            content_dict = resource_content.model_dump()
                                        elif hasattr(resource_content, "dict"):
                                            content_dict = resource_content.dict()
                                        else:
                                            content_dict = {"content_type": "unknown"}
                                    
                                    mime_type = resource.get("mime_type", "")
                                    if "json" in mime_type and "content" in content_dict:
                                        try:
                                            # Parse and pretty-print JSON content
                                            if isinstance(content_dict["content"], str):
                                                parsed_json = json.loads(content_dict["content"])
                                                log.info(f"Resource content (JSON): {json.dumps(parsed_json, indent=2)}")
                                            else:
                                                log.info(f"Resource content: {json.dumps(content_dict, indent=2)}")
                                        except:
                                            log.info(f"Resource content: {content_dict}")
                                    else:
                                        # Just print the content normally
                                        log.info(f"Resource content: {content_dict}")
                                else:
                                    log.error(f"Failed to read resource: {uri}")
                except Exception as e:
                    log.error(f"Error processing resources: {e}")
                    if self.debug_mode:
                        traceback.print_exc()
            else:
                log.error("Failed to retrieve resources list")
            
            # List tools
            log.info("--- Listing Available Tools ---")
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
                    if tool.name == "calculator" or tool.name == "Calculator":
                        calculator_tool = tool
                        break

                
                if calculator_tool:
                    log.info(f"Found calculator tool: {calculator_tool.description}")
                    
                    # Get the input schema from the tool
                    input_schema = {}
                    if hasattr(calculator_tool, "inputSchema"):
                        input_schema = calculator_tool.inputSchema
                    
                    log.info("Input schema: " + json.dumps(input_schema, indent=2))
                    
                    # Extract valid operations from the schema
                    valid_operations = []
                    try:
                        if isinstance(input_schema, dict) and "properties" in input_schema:
                            if "operation" in input_schema["properties"]:
                                operation_schema = input_schema["properties"]["operation"]
                                if "enum" in operation_schema:
                                    valid_operations = operation_schema["enum"]
                    except Exception as e:
                        log.error(f"Error extracting operations from schema: {e}")
                    
                    if not valid_operations:
                        # Fallback to defaults if we couldn't extract operations
                        valid_operations = ["add", "subtract", "multiply", "divide"]
                        
                    log.info(f"Valid operations from schema: {valid_operations}")
                    
                    # Build test cases using operations from the schema
                    operations = []
                    if len(valid_operations) >= 4:
                        operations = [
                            {"operation": valid_operations[0], "a": 5, "b": 3},
                            {"operation": valid_operations[1], "a": 10, "b": 4},
                            {"operation": valid_operations[2], "a": 6, "b": 7},
                            {"operation": valid_operations[3], "a": 15, "b": 3}
                        ]
                    else:
                        # If we don't have enough operations, use the first one multiple times
                        op = valid_operations[0] if valid_operations else "add"
                        operations = [
                            {"operation": op, "a": 5, "b": 3},
                            {"operation": op, "a": 10, "b": 4},
                            {"operation": op, "a": 6, "b": 7},
                            {"operation": op, "a": 15, "b": 3}
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