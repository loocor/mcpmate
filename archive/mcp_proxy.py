#!/usr/bin/env python3
"""
MCP Proxy Server

A proxy server that exposes tools from other enabled MCP servers.
"""

import asyncio
import json
import logging
import os
import sys
import time
from logging.handlers import RotatingFileHandler
from typing import Any, Dict, List, Iterable

from mcp_client import MCPClient, format_result
import mcp.types as types
from mcp.server.lowlevel.server import Server
from mcp.server.stdio import stdio_server

# Create logs directory if it doesn't exist
log_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "logs")
os.makedirs(log_dir, exist_ok=True)

# Configure logging
log_file = os.path.join(log_dir, "mcp_proxy.log")

# Configure basic logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s"
)

# Create file handler
file_handler = RotatingFileHandler(
    log_file,
    maxBytes=10 * 1024 * 1024,  # 10 MB
    backupCount=5,
    encoding="utf-8"
)

# Configure log format
log_format = logging.Formatter(
    "%(asctime)s - %(levelname)s - %(message)s"
)
file_handler.setFormatter(log_format)

# Get root logger and add file handler
logger = logging.getLogger(__name__)
logger.addHandler(file_handler)

class MCPProxyServer:
    """A proxy server that exposes tools from other enabled MCP servers."""

    def __init__(self, config_path: str = "mcp.json"):
        """Initialize the MCP proxy server.

        Args:
            config_path: Path to the MCP configuration file.
        """
        self.config_path = config_path
        self.client = MCPClient(config_path)
        self.enabled_servers = self.client.get_server_names(only_enabled=True)
        # Filter out the proxy server itself
        self.enabled_servers = [name for name in self.enabled_servers if name != "proxy"]
        self.server_sessions = {}
        self.server_tools = {}

        # Create an MCP server
        self.server = Server("proxy", "1.0.0", "MCP Proxy Server that exposes tools from other enabled MCP servers")

        # Register handlers
        @self.server.list_tools()
        async def handle_list_tools() -> List[types.Tool]:
            try:
                logger.info("Received list_tools request")
                # Connect to enabled servers if not already connected
                if not self.server_sessions:
                    logger.info("No active server sessions, connecting to enabled servers")
                    await self.connect_to_enabled_servers()
                else:
                    logger.info(f"Using existing connections to {len(self.server_sessions)} servers")

                # Convert tool info to Tool objects
                tools = []
                for _, server_tools in self.server_tools.items():  # Use _ to ignore unused variable
                    for tool in server_tools:
                        # Create a Tool object
                        tool_obj = types.Tool(
                            name=tool["name"],
                            description=tool["description"],
                            inputSchema=tool.get("input_schema", {})
                        )
                        tools.append(tool_obj)

                logger.info(f"Returning {len(tools)} tools")
                return tools
            except Exception as e:
                logger.error(f"Error in handle_list_tools: {e}", exc_info=True)
                # Return an empty list rather than failing
                return []

        @self.server.call_tool()
        async def handle_call_tool(name: str, arguments: Dict[str, Any]) -> Iterable[types.TextContent]:
            try:
                logger.info(f"Received call_tool request for tool '{name}'")
                logger.info(f"Calling tool: {name}")
                logger.info(f"Arguments: {json.dumps(arguments, indent=2)}")

                # Call the tool on the appropriate server
                start_time = time.time()
                result = await self.call_tool(name, arguments)
                elapsed_time = time.time() - start_time

                logger.info(f"Tool '{name}' call completed in {elapsed_time:.2f} seconds")
                logger.info(f"Raw result type: {type(result)}")

                # Check if the result is already a TextContent object or list of TextContent objects
                if isinstance(result, types.TextContent):
                    logger.info(f"Result is already a TextContent object: {result.text[:100]}...")
                    return [result]
                elif isinstance(result, list) and all(isinstance(item, types.TextContent) for item in result):
                    logger.info(f"Result is already a list of TextContent objects with {len(result)} items")
                    return result

                # Convert the result to TextContent
                formatted_result = format_result(result)
                result_length = len(formatted_result) if formatted_result else 0
                logger.info(f"Tool '{name}' call successful, result length: {result_length}")

                # For very large results, log a preview
                if result_length > 1000:
                    preview = formatted_result[:500] + "..." + formatted_result[-500:]
                    logger.info(f"Result preview: {preview}")
                else:
                    logger.info(f"Full result: {formatted_result}")

                # Ensure the result is properly formatted as TextContent
                if not formatted_result:
                    logger.warning(f"Empty result from tool '{name}'")
                    return [types.TextContent(type="text", text="")]

                # Create a TextContent object with the formatted result
                text_content = types.TextContent(type="text", text=formatted_result)
                logger.info(f"Created TextContent object with type: {text_content.type}")

                return [text_content]
            except Exception as e:
                error_msg = f"Error calling tool '{name}': {str(e)}"
                logger.error(error_msg, exc_info=True)
                # Return the error as TextContent
                return [types.TextContent(type="text", text=f"Error: {str(e)}")]

        # Suppress IDE warnings about unused functions
        _ = handle_list_tools
        _ = handle_call_tool

    async def connect_to_enabled_servers(self) -> None:
        """Connect to all enabled servers."""
        # First disconnect from any existing servers to avoid resource leaks
        if self.server_sessions:
            logger.info("Disconnecting from existing servers before reconnecting")
            await self.disconnect_from_servers()

        # Connect to each enabled server
        for server_name in self.enabled_servers:
            try:
                logger.info(f"Connecting to server: {server_name}")

                # Use a timeout to avoid hanging on connection
                try:
                    connect_task = asyncio.create_task(self._connect_to_server(server_name))
                    await asyncio.wait_for(connect_task, timeout=30.0)
                except asyncio.TimeoutError:
                    logger.error(f"Timeout connecting to server '{server_name}'")
                except Exception as connect_error:
                    logger.error(f"Error in connection task for server '{server_name}': {connect_error}")
            except Exception as e:
                logger.error(f"Error connecting to server '{server_name}': {e}")

    async def _connect_to_server(self, server_name: str) -> None:
        """Helper method to connect to a server in a separate task."""
        try:
            logger.info(f"Connecting to server: {server_name}")

            start_time = time.time()
            result = await self.client.connect_to_server(server_name)
            elapsed_time = time.time() - start_time

            if result:
                session, stdio_ctx, session_ctx = result
                self.server_sessions[server_name] = (session, stdio_ctx, session_ctx)

                logger.info(f"Connected to server '{server_name}' in {elapsed_time:.2f} seconds")

                # Get tools from this server
                tools_start_time = time.time()
                tools = await self.client.list_tools(session)
                tools_elapsed_time = time.time() - tools_start_time

                self.server_tools[server_name] = tools

                logger.info(f"Retrieved {len(tools)} tools from server '{server_name}' in {tools_elapsed_time:.2f} seconds")
                for i, tool in enumerate(tools):
                    logger.info(f"  Tool {i+1}: {tool['name']} - {tool['description'][:100]}...")
            else:
                logger.error(f"Failed to connect to server: {server_name}")
        except Exception as e:
            logger.error(f"Error in _connect_to_server for '{server_name}': {e}", exc_info=True)

    async def disconnect_from_servers(self) -> None:
        """Disconnect from all servers."""
        # Make a copy of the items to avoid modifying during iteration
        server_items = list(self.server_sessions.items())

        for server_name, (_, stdio_ctx, session_ctx) in server_items:
            try:
                logger.info(f"Disconnecting from server: {server_name}")

                # First remove from our dictionaries to prevent further use
                if server_name in self.server_sessions:
                    del self.server_sessions[server_name]
                if server_name in self.server_tools:
                    del self.server_tools[server_name]

                # Use a timeout to avoid hanging
                try:
                    # Create a separate task for closing contexts
                    close_task = asyncio.create_task(self._close_contexts(stdio_ctx, session_ctx, server_name))
                    await asyncio.wait_for(close_task, timeout=5.0)
                except asyncio.TimeoutError:
                    logger.warning(f"Timeout while disconnecting from server '{server_name}'")
                except Exception as ctx_error:
                    logger.warning(f"Context exit error for server '{server_name}': {ctx_error}")
            except Exception as e:
                logger.warning(f"Error disconnecting from server '{server_name}': {e}")
                # Make sure dictionaries are clean
                if server_name in self.server_sessions:
                    del self.server_sessions[server_name]
                if server_name in self.server_tools:
                    del self.server_tools[server_name]

    async def _close_contexts(self, stdio_ctx, session_ctx, server_name):
        """Helper method to close contexts in a separate task."""
        try:
            await session_ctx.__aexit__(None, None, None)
            await stdio_ctx.__aexit__(None, None, None)
            logger.info(f"Successfully closed contexts for server '{server_name}'")
        except Exception as e:
            logger.warning(f"Error closing contexts for server '{server_name}': {e}")

    def get_all_tools(self) -> List[Dict[str, Any]]:
        """Get all tools from all enabled servers.

        Returns:
            A list of tools.
        """
        all_tools = []
        for server_name, tools in self.server_tools.items():
            for tool in tools:
                # Add server name to tool info
                tool["server"] = server_name
                all_tools.append(tool)
        return all_tools

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """Call a tool on the appropriate server.

        Args:
            tool_name: The name of the tool to call.
            arguments: The arguments to pass to the tool.

        Returns:
            The result of the tool call.

        Raises:
            ValueError: If the tool is not found on any server.
        """
        # Find which server has this tool
        server_name = None
        for s_name, tools in self.server_tools.items():
            for tool in tools:
                if tool["name"] == tool_name:
                    server_name = s_name
                    break
            if server_name:
                break

        if not server_name:
            raise ValueError(f"Tool '{tool_name}' not found on any enabled server")

        logger.info(f"Found tool '{tool_name}' on server '{server_name}'")

        # Check if we have a session for this server
        if server_name not in self.server_sessions:
            logger.warning(f"No active session for server '{server_name}', reconnecting...")
            await self._connect_to_server(server_name)

            # Check if connection was successful
            if server_name not in self.server_sessions:
                raise ValueError(f"Failed to connect to server '{server_name}'")

        # Call the tool on this server
        session = self.server_sessions[server_name][0]

        try:
            # Call the tool and get the result
            result = await self.client.call_tool(session, tool_name, arguments)

            # Log the result type for debugging
            logger.info(f"Raw result type from server '{server_name}': {type(result)}")

            return result
        except Exception as e:
            logger.error(f"Error calling tool '{tool_name}' on server '{server_name}': {e}", exc_info=True)

            # Try to reconnect and retry once
            try:
                logger.info(f"Reconnecting to server '{server_name}' and retrying...")
                await self._connect_to_server(server_name)

                # Check if reconnection was successful
                if server_name not in self.server_sessions:
                    raise ValueError(f"Failed to reconnect to server '{server_name}'")

                # Retry the tool call
                session = self.server_sessions[server_name][0]
                return await self.client.call_tool(session, tool_name, arguments)
            except Exception as retry_error:
                logger.error(f"Error retrying tool '{tool_name}' on server '{server_name}': {retry_error}", exc_info=True)
                raise

async def main() -> None:
    """Main entry point for the MCP proxy server."""
    import argparse

    parser = argparse.ArgumentParser(description="MCP Proxy Server")
    parser.add_argument("--config", default="mcp.json", help="Path to the MCP configuration file (default: mcp.json)")
    parser.add_argument("--stdio", action="store_true", help="Run in stdio mode (for MCP client)")
    parser.add_argument("--debug", action="store_true", help="Enable debug logging")
    parser.add_argument("--log-file", default=None, help="Path to the log file (default: logs/mcp_proxy.log)")

    args = parser.parse_args()
    config_path = args.config
    stdio_mode = args.stdio

    # Configure logging
    log_level = logging.DEBUG if args.debug else logging.INFO
    logger.setLevel(log_level)

    # If a custom log file is specified, use it
    if args.log_file:
        # Remove existing file handlers
        for handler in logger.handlers[:]:
            if isinstance(handler, RotatingFileHandler):
                logger.removeHandler(handler)

        # Create a new log directory if needed
        log_dir = os.path.dirname(args.log_file)
        if log_dir:
            os.makedirs(log_dir, exist_ok=True)

        # Create a new file handler
        custom_file_handler = RotatingFileHandler(
            args.log_file,
            maxBytes=10 * 1024 * 1024,  # 10 MB
            backupCount=5,
            encoding="utf-8"
        )
        custom_file_handler.setFormatter(log_format)
        logger.addHandler(custom_file_handler)

    logger.info(f"Starting MCP Proxy Server with log level: {logging.getLevelName(log_level)}")
    logger.info(f"Log file: {args.log_file or log_file}")

    # Log system information
    logger.info(f"Python version: {sys.version}")
    logger.info(f"Current working directory: {os.getcwd()}")
    logger.info(f"Script directory: {os.path.dirname(os.path.abspath(__file__))}")
    logger.info(f"Configuration file: {config_path}")

    if args.debug:
        logger.debug("Debug logging enabled")

    proxy = None
    try:
        # Create the proxy server
        proxy = MCPProxyServer(config_path)

        if stdio_mode:
            # Run as an MCP server over stdio
            print("Starting MCP proxy server in stdio mode...", file=sys.stderr)
            logger.info("Starting in stdio mode")

            # Pre-connect to all enabled servers
            logger.info("Pre-connecting to enabled servers")
            await proxy.connect_to_enabled_servers()

            try:
                # Use a separate task for the stdio server to avoid task/scope issues
                async def run_stdio_server():
                    try:
                        async with stdio_server() as (read_stream, write_stream):
                            # Create initialization options
                            init_options = proxy.server.create_initialization_options()

                            # Run the server
                            logger.info("Starting MCP server with stdio transport")
                            await proxy.server.run(read_stream, write_stream, init_options)
                    except Exception as stdio_error:
                        logger.error(f"Error in stdio server: {stdio_error}")
                        raise

                # Run the stdio server with a timeout
                await asyncio.wait_for(run_stdio_server(), timeout=3600.0)  # 1 hour timeout
            except asyncio.TimeoutError:
                logger.error("Timeout in stdio server")
            except Exception as e:
                logger.error(f"Error in stdio mode: {e}")
            finally:
                # Clean up resources
                if proxy:
                    logger.info("Cleaning up resources")
                    await proxy.disconnect_from_servers()
        else:
            # Connect to all enabled servers
            await proxy.connect_to_enabled_servers()

            # Print all available tools
            all_tools = proxy.get_all_tools()
            print(f"\nAvailable tools from {len(proxy.enabled_servers)} enabled servers:")
            for tool in all_tools:
                print(f"  - {tool['name']} (from {tool['server']}): {tool['description']}")

            # Keep the server running
            try:
                print("\nProxy server is running. Press Ctrl+C to exit.")
                while True:
                    await asyncio.sleep(1)
            except KeyboardInterrupt:
                print("\nShutting down...")
            finally:
                # Disconnect from all servers
                if proxy:
                    logger.info("Disconnecting from servers")
                    await proxy.disconnect_from_servers()

    except Exception as e:
        logger.error(f"Error in main: {e}", exc_info=True)
        # Make sure we clean up
        if proxy:
            try:
                await proxy.disconnect_from_servers()
            except Exception as cleanup_error:
                logger.error(f"Error during cleanup: {cleanup_error}")
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
