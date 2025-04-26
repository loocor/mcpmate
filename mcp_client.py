#!/usr/bin/env python3
"""
MCP Common - Common utilities for MCP demos.
"""

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

    def __init__(self, config_path: str = "mcp.json"):
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
        # Try to find the configuration file in the current directory
        # or in the parent directory (project root)
        paths_to_try = [
            self.config_path,  # Current directory
            os.path.join("..", self.config_path),  # Parent directory
            os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), self.config_path)  # Absolute path to project root
        ]

        for path in paths_to_try:
            try:
                with open(path, 'r') as f:
                    logger.info(f"Loading configuration from: {path}")
                    return json.load(f)
            except FileNotFoundError:
                continue
            except json.JSONDecodeError:
                logger.error(f"Invalid JSON in configuration file: {path}")
                raise

        # If we get here, we couldn't find the configuration file
        logger.error(f"Configuration file not found: {self.config_path}")
        logger.error(f"Tried paths: {paths_to_try}")
        raise FileNotFoundError(f"Configuration file not found: {self.config_path}")

    def get_server_names(self) -> List[str]:
        """Get the names of all configured servers.

        Returns:
            A list of server names.
        """
        return list(self.config.get("mcpServers", {}).keys())

    def get_server_config(self, server_name: str) -> Dict[str, Any]:
        """Get the configuration for a server.

        Args:
            server_name: The name of the server.

        Returns:
            The server configuration.

        Raises:
            ValueError: If the server name is not found in the configuration.
        """
        if server_name not in self.get_server_names():
            raise ValueError(f"Server '{server_name}' not found in configuration")

        return self.config["mcpServers"][server_name]

    async def connect_to_server(self, server_name: str) -> Optional[Tuple[ClientSession, Any, Any]]:
        """Connect to an MCP server.

        Args:
            server_name: The name of the server to connect to.

        Returns:
            A tuple of (session, stdio_context, session_context), or None if connection failed.

        Raises:
            ValueError: If the server name is not found in the configuration.
        """
        server_config = self.get_server_config(server_name)

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
            stdio_context = stdio_client(server_params)
            read, write = await stdio_context.__aenter__()

            session_context = ClientSession(read, write)
            session = await session_context.__aenter__()

            # Initialize the connection
            await session.initialize()

            return session, stdio_context, session_context
        except Exception as e:
            logger.error(f"Error connecting to server '{server_name}': {e}")
            return None

    async def list_tools(self, session: ClientSession) -> List[Dict[str, Any]]:
        """List all tools available on the server.

        Args:
            session: The MCP client session.

        Returns:
            A list of tools.
        """
        tools = []
        try:
            tools_response = await session.list_tools()
            for tool_item in tools_response:
                if isinstance(tool_item, tuple) and tool_item[0] == "tools":
                    for tool in tool_item[1]:
                        tool_info = {
                            "name": tool.name,
                            "description": tool.description,
                            "input_schema": tool.inputSchema if hasattr(tool, "inputSchema") else {}
                        }
                        tools.append(tool_info)
        except Exception as e:
            logger.warning(f"Error listing tools: {e}")

        return tools

    async def list_resources(self, session: ClientSession) -> List[Dict[str, Any]]:
        """List all resources available on the server.

        Args:
            session: The MCP client session.

        Returns:
            A list of resources.
        """
        resources = []
        try:
            resources_response = await session.list_resources()
            for resource_item in resources_response:
                if isinstance(resource_item, tuple) and resource_item[0] == "resources":
                    for resource in resource_item[1]:
                        resource_info = {
                            "pattern": resource.pattern if hasattr(resource, "pattern") else str(resource),
                            "description": resource.description if hasattr(resource, "description") else ""
                        }
                        resources.append(resource_info)
        except Exception as e:
            logger.warning(f"Error listing resources: {e}")

        return resources

    async def list_prompts(self, session: ClientSession) -> List[Dict[str, Any]]:
        """List all prompts available on the server.

        Args:
            session: The MCP client session.

        Returns:
            A list of prompts.
        """
        prompts = []
        try:
            prompts_response = await session.list_prompts()
            for prompt in prompts_response:
                prompt_info = {
                    "name": prompt.name,
                    "description": prompt.description,
                    "arguments": [
                        {
                            "name": arg.name,
                            "description": arg.description,
                            "required": arg.required
                        }
                        for arg in prompt.arguments
                    ] if hasattr(prompt, "arguments") else []
                }
                prompts.append(prompt_info)
        except Exception as e:
            logger.warning(f"Error listing prompts: {e}")

        return prompts

    async def get_server_info(self, session: ClientSession) -> Dict[str, Any]:
        """Get information from an MCP server.

        Args:
            session: The MCP client session.

        Returns:
            A dictionary containing server information.
        """
        server_info = {}

        # Get tools
        server_info["tools"] = await self.list_tools(session)

        # Get resources
        server_info["resources"] = await self.list_resources(session)

        # Get prompts
        server_info["prompts"] = await self.list_prompts(session)

        return server_info

    async def call_tool(self, session: ClientSession, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """Call a tool on the server.

        Args:
            session: The MCP client session.
            tool_name: The name of the tool to call.
            arguments: The arguments to pass to the tool.

        Returns:
            The result of the tool call.

        Raises:
            Exception: If the tool call fails.
        """
        logger.info(f"Calling tool: {tool_name}")
        logger.info(f"Arguments: {json.dumps(arguments, indent=2)}")

        try:
            result = await session.call_tool(tool_name, arguments)
            return result
        except Exception as e:
            logger.error(f"Error calling tool: {e}")
            raise

    def print_server_info(self, server_name: str, server_info: Dict[str, Any]) -> None:
        """Print server information to the console.

        Args:
            server_name: The name of the server.
            server_info: The server information.
        """
        print(f"\n{'=' * 50}")
        print(f"Server: {server_name}")
        print(f"{'=' * 50}")

        # Print tools
        tools = server_info.get("tools", [])
        print(f"\nTools ({len(tools)}):")
        print("-" * 50)
        if tools:
            for tool in tools:
                print(f"  - {tool['name']}: {tool['description']}")
                if tool.get("input_schema") and tool["input_schema"].get("properties"):
                    print("    Arguments:")
                    for param_name, param_info in tool["input_schema"]["properties"].items():
                        required = "required" if param_name in tool["input_schema"].get("required", []) else "optional"
                        desc = param_info.get("description", "No description")
                        print(f"      {param_name} ({required}): {desc}")
        else:
            print("  No tools available")

        # Print resources
        resources = server_info.get("resources", [])
        print(f"\nResources ({len(resources)}):")
        print("-" * 50)
        if resources:
            for resource in resources:
                print(f"  - {resource['pattern']}: {resource['description']}")
        else:
            print("  No resources available")

        # Print prompts
        prompts = server_info.get("prompts", [])
        print(f"\nPrompts ({len(prompts)}):")
        print("-" * 50)
        if prompts:
            for prompt in prompts:
                print(f"  - {prompt['name']}: {prompt['description']}")
                if prompt.get("arguments"):
                    print("    Arguments:")
                    for arg in prompt["arguments"]:
                        required = "required" if arg["required"] else "optional"
                        print(f"      {arg['name']} ({required}): {arg['description']}")
        else:
            print("  No prompts available")

async def run_with_server(server_name: str, callback: callable, config_path: str = "mcp.json") -> None:
    """Run a callback function with a connected server.

    Args:
        server_name: The name of the server to connect to.
        callback: The callback function to run with the server session.
        config_path: Path to the MCP configuration file.
    """
    client = MCPClient(config_path)

    # Connect to the server
    result = await client.connect_to_server(server_name)
    if not result:
        logger.error(f"Failed to connect to server: {server_name}")
        sys.exit(1)

    session, stdio_ctx, session_ctx = result

    try:
        # Run the callback function
        await callback(session)
    finally:
        # Close the session and contexts
        try:
            await session_ctx.__aexit__(None, None, None)
            await stdio_ctx.__aexit__(None, None, None)
        except Exception as e:
            logger.warning(f"Error closing session for server '{server_name}': {e}")

def format_result(result: Any) -> str:
    """Format a result for display.

    Args:
        result: The result to format.

    Returns:
        A formatted string representation of the result.
    """
    if isinstance(result, dict):
        return json.dumps(result, indent=2)
    elif hasattr(result, 'content') and hasattr(result.content[0], 'text'):
        return result.content[0].text
    elif hasattr(result, '__dict__'):
        try:
            result_dict = result.__dict__
            return json.dumps(result_dict, indent=2)
        except Exception:
            return str(result)
    else:
        return str(result)

def parse_arguments() -> Dict[str, Any]:
    """Parse command line arguments.

    Returns:
        A dictionary of parsed arguments.
    """
    import argparse

    parser = argparse.ArgumentParser(description="MCP Demo")
    parser.add_argument("--config", default="mcp.json", help="Path to the MCP configuration file (default: mcp.json)")
    parser.add_argument("--server", help="Name of the server to connect to (if not specified, will list available servers)")
    parser.add_argument("--list", action="store_true", help="List available servers")
    parser.add_argument("--all", action="store_true", help="Connect to all servers")

    return vars(parser.parse_args())
