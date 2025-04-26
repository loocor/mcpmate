#!/usr/bin/env python3
"""
MCP Proxy Server

A proxy server that exposes tools from other enabled MCP servers.
"""

import asyncio
import logging
import sys
from typing import Any, Dict, List, Iterable

from mcp_client import MCPClient, format_result
import mcp.types as types
from mcp.server.lowlevel.server import Server
from mcp.server.stdio import stdio_server

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

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
            # Connect to enabled servers if not already connected
            if not self.server_sessions:
                await self.connect_to_enabled_servers()

            # Convert tool info to Tool objects
            tools = []
            for server_name, server_tools in self.server_tools.items():
                for tool in server_tools:
                    # Create a Tool object
                    tool_obj = types.Tool(
                        name=tool["name"],
                        description=tool["description"],
                        inputSchema=tool.get("input_schema", {})
                    )
                    tools.append(tool_obj)

            return tools

        @self.server.call_tool()
        async def handle_call_tool(name: str, arguments: Dict[str, Any]) -> Iterable[types.TextContent]:
            try:
                # Call the tool on the appropriate server
                result = await self.call_tool(name, arguments)

                # Convert the result to TextContent
                return [types.TextContent(type="text", text=format_result(result))]
            except Exception as e:
                # Return the error as TextContent
                return [types.TextContent(type="text", text=f"Error: {str(e)}")]

    async def connect_to_enabled_servers(self) -> None:
        """Connect to all enabled servers."""
        for server_name in self.enabled_servers:
            try:
                logger.info(f"Connecting to server: {server_name}")
                result = await self.client.connect_to_server(server_name)
                if result:
                    session, stdio_ctx, session_ctx = result
                    self.server_sessions[server_name] = (session, stdio_ctx, session_ctx)

                    # Get tools from this server
                    tools = await self.client.list_tools(session)
                    self.server_tools[server_name] = tools
                    logger.info(f"Connected to server: {server_name}, found {len(tools)} tools")
                else:
                    logger.error(f"Failed to connect to server: {server_name}")
            except Exception as e:
                logger.error(f"Error connecting to server '{server_name}': {e}")

    async def disconnect_from_servers(self) -> None:
        """Disconnect from all servers."""
        for server_name, (_, stdio_ctx, session_ctx) in self.server_sessions.items():
            try:
                logger.info(f"Disconnecting from server: {server_name}")
                await session_ctx.__aexit__(None, None, None)
                await stdio_ctx.__aexit__(None, None, None)
            except Exception as e:
                logger.warning(f"Error disconnecting from server '{server_name}': {e}")

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
        for server_name, tools in self.server_tools.items():
            for tool in tools:
                if tool["name"] == tool_name:
                    # Call the tool on this server
                    session = self.server_sessions[server_name][0]
                    return await self.client.call_tool(session, tool_name, arguments)

        # If we get here, the tool was not found
        raise ValueError(f"Tool '{tool_name}' not found on any enabled server")

async def main() -> None:
    """Main entry point for the MCP proxy server."""
    import argparse

    parser = argparse.ArgumentParser(description="MCP Proxy Server")
    parser.add_argument("--config", default="mcp.json", help="Path to the MCP configuration file (default: mcp.json)")
    parser.add_argument("--stdio", action="store_true", help="Run in stdio mode (for MCP client)")

    args = parser.parse_args()
    config_path = args.config
    stdio_mode = args.stdio

    try:
        proxy = MCPProxyServer(config_path)

        if stdio_mode:
            # Run as an MCP server over stdio
            print("Starting MCP proxy server in stdio mode...", file=sys.stderr)
            async with stdio_server() as (read_stream, write_stream):
                # Create initialization options
                init_options = proxy.server.create_initialization_options()

                # Run the server
                await proxy.server.run(read_stream, write_stream, init_options)
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
                await proxy.disconnect_from_servers()

    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
