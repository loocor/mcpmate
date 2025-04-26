#!/usr/bin/env python3
"""
MCP Client - A simple command line tool to interact with MCP servers.
"""

import asyncio
import argparse
import json
import logging
import os
import sys
from typing import Any, Dict, List, Optional, Tuple

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

class MCPClient:
    """A client for interacting with MCP servers."""

    def __init__(self, config_path: str):
        """Initialize the MCP client.

        Args:
            config_path: Path to the MCP configuration file.
        """
        self.config_path = config_path
        self.config = self._load_config()

    def _load_config(self) -> Dict[str, Any]:
        """Load the MCP configuration from a file.

        Returns:
            The loaded configuration.

        Raises:
            FileNotFoundError: If the configuration file doesn't exist.
            json.JSONDecodeError: If the configuration file is invalid JSON.
        """
        try:
            with open(self.config_path, 'r') as f:
                return json.load(f)
        except FileNotFoundError:
            logger.error(f"Configuration file not found: {self.config_path}")
            raise
        except json.JSONDecodeError:
            logger.error(f"Invalid JSON in configuration file: {self.config_path}")
            raise

    def get_server_names(self) -> List[str]:
        """Get the names of all configured servers.

        Returns:
            A list of server names.
        """
        return list(self.config.get("mcpServers", {}).keys())

    async def connect_to_server(self, server_name: str) -> None:
        """Connect to an MCP server and retrieve information.

        Args:
            server_name: The name of the server to connect to.

        Raises:
            ValueError: If the server name is not found in the configuration.
        """
        if server_name not in self.get_server_names():
            raise ValueError(f"Server '{server_name}' not found in configuration")

        server_config = self.config["mcpServers"][server_name]

        # Create server parameters
        command = server_config["command"]
        args = server_config.get("args", [])
        env = server_config.get("env", {})

        # Merge environment variables
        full_env = os.environ.copy()
        if env:
            full_env.update(env)

        server_params = StdioServerParameters(
            command=command,
            args=args,
            env=full_env
        )

        logger.info(f"Connecting to server: {server_name}")
        logger.info(f"Command: {command} {' '.join(args)}")

        try:
            async with stdio_client(server_params) as (read, write):
                async with ClientSession(read, write) as session:
                    # Initialize the connection
                    await session.initialize()

                    # Get server information
                    server_info = await self._get_server_info(session)

                    # Print server information
                    self._print_server_info(server_name, server_info)
        except Exception as e:
            logger.error(f"Error connecting to server '{server_name}': {e}")
            raise

    async def _get_server_info(self, session: ClientSession) -> Dict[str, Any]:
        """Get information from an MCP server.

        Args:
            session: The MCP client session.

        Returns:
            A dictionary containing server information.
        """
        server_info = {}

        # Get server capabilities
        try:
            capabilities = session.capabilities
            server_info["capabilities"] = capabilities
            logger.info(f"Server capabilities: {capabilities}")
        except Exception as e:
            logger.warning(f"Error getting capabilities: {e}")
            server_info["capabilities"] = {}

        # Get tools
        try:
            tools = await session.list_tools()
            server_info["tools"] = tools
        except Exception as e:
            logger.warning(f"Error listing tools: {e}")
            server_info["tools"] = []

        # Get resources
        try:
            # Only try to list resources if the server has the capability
            if server_info.get("capabilities", {}).get("resources", False):
                resources = await session.list_resources()
                server_info["resources"] = resources
            else:
                logger.info("Server does not support resources")
                server_info["resources"] = []
        except Exception as e:
            logger.warning(f"Error listing resources: {e}")
            server_info["resources"] = []

        # Get prompts
        try:
            # Only try to list prompts if the server has the capability
            if server_info.get("capabilities", {}).get("prompts", False):
                prompts = await session.list_prompts()
                server_info["prompts"] = prompts
            else:
                logger.info("Server does not support prompts")
                server_info["prompts"] = []
        except Exception as e:
            logger.warning(f"Error listing prompts: {e}")
            server_info["prompts"] = []

        return server_info

    def _print_server_info(self, server_name: str, server_info: Dict[str, Any]) -> None:
        """Print server information to the console.

        Args:
            server_name: The name of the server.
            server_info: The server information.
        """
        print(f"\n{'=' * 50}")
        print(f"Server: {server_name}")
        print(f"{'=' * 50}")

        # Print capabilities
        capabilities = server_info.get("capabilities", {})
        print("\nCapabilities:")
        print("-" * 50)
        if capabilities:
            for cap_name, cap_value in capabilities.items():
                if isinstance(cap_value, dict):
                    print(f"  - {cap_name}:")
                    for sub_name, sub_value in cap_value.items():
                        print(f"      {sub_name}: {sub_value}")
                else:
                    print(f"  - {cap_name}: {cap_value}")
        else:
            print("  No capabilities available")

        # Print tools
        tools = server_info.get("tools", [])
        tool_count = 0
        for tool_item in tools:
            if isinstance(tool_item, tuple) and tool_item[0] == "tools":
                tool_count = len(tool_item[1])

        print(f"\nTools ({tool_count}):")
        print("-" * 50)
        if tools:
            for tool_item in tools:
                if isinstance(tool_item, tuple) and tool_item[0] == "tools":
                    for tool in tool_item[1]:
                        print(f"  - {tool.name}: {tool.description}")
                        if hasattr(tool, 'inputSchema') and tool.inputSchema:
                            print("    Arguments:")
                            if "properties" in tool.inputSchema:
                                for param_name, param_info in tool.inputSchema["properties"].items():
                                    required = "required" if param_name in tool.inputSchema.get("required", []) else "optional"
                                    desc = param_info.get("description", "No description")
                                    print(f"      {param_name} ({required}): {desc}")
        else:
            print("  No tools available")

        # Print resources
        resources = server_info.get("resources", [])
        resource_count = 0
        for resource_item in resources:
            if isinstance(resource_item, tuple) and resource_item[0] == "resources":
                resource_count = len(resource_item[1])

        print(f"\nResources ({resource_count}):")
        print("-" * 50)
        if resources:
            for resource_item in resources:
                if isinstance(resource_item, tuple) and resource_item[0] == "resources":
                    for resource in resource_item[1]:
                        print(f"  - {resource.pattern}: {resource.description}")
        else:
            print("  No resources available")

        # Print prompts
        prompts = server_info.get("prompts", [])
        print(f"\nPrompts ({len(prompts)}):")
        print("-" * 50)
        if prompts:
            for prompt in prompts:
                print(f"  - {prompt.name}: {prompt.description}")
                if hasattr(prompt, 'arguments') and prompt.arguments:
                    print("    Arguments:")
                    for arg in prompt.arguments:
                        required = "required" if arg.required else "optional"
                        print(f"      {arg.name} ({required}): {arg.description}")
        else:
            print("  No prompts available")

async def main():
    """Main entry point for the MCP client."""
    parser = argparse.ArgumentParser(description="MCP Client - A simple command line tool to interact with MCP servers")
    parser.add_argument("--config", default="mcp.json", help="Path to the MCP configuration file (default: mcp.json)")
    parser.add_argument("--server", help="Name of the server to connect to (if not specified, will list available servers)")
    parser.add_argument("--list", action="store_true", help="List available servers")
    parser.add_argument("--all", action="store_true", help="Connect to all servers")

    args = parser.parse_args()

    try:
        client = MCPClient(args.config)

        if args.list:
            # List available servers
            servers = client.get_server_names()
            print("Available servers:")
            for server in servers:
                print(f"  - {server}")
            return

        if args.all:
            # Connect to all servers
            servers = client.get_server_names()
            for server in servers:
                try:
                    await client.connect_to_server(server)
                except Exception as e:
                    logger.error(f"Error connecting to server '{server}': {e}")
            return

        if args.server:
            # Connect to the specified server
            await client.connect_to_server(args.server)
            return

        # If no specific action is requested, list servers and prompt for selection
        servers = client.get_server_names()
        if not servers:
            print("No servers found in configuration")
            return

        print("Available servers:")
        for i, server in enumerate(servers, 1):
            print(f"  {i}. {server}")

        try:
            choice = input("\nEnter server number to connect to (or 'all' for all servers): ")
            if choice.lower() == 'all':
                for server in servers:
                    try:
                        await client.connect_to_server(server)
                    except Exception as e:
                        logger.error(f"Error connecting to server '{server}': {e}")
            else:
                idx = int(choice) - 1
                if 0 <= idx < len(servers):
                    await client.connect_to_server(servers[idx])
                else:
                    print("Invalid selection")
        except ValueError:
            print("Invalid input")
        except KeyboardInterrupt:
            print("\nOperation cancelled")

    except FileNotFoundError:
        print(f"Configuration file not found: {args.config}")
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"Invalid JSON in configuration file: {args.config}")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
