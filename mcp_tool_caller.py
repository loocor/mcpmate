#!/usr/bin/env python3
"""
MCP Tool Caller - A command line tool to call tools on MCP servers.
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

class MCPToolCaller:
    """A client for calling tools on MCP servers."""
    
    def __init__(self, config_path: str):
        """Initialize the MCP tool caller.
        
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
    
    async def connect_to_server(self, server_name: str) -> Optional[ClientSession]:
        """Connect to an MCP server.
        
        Args:
            server_name: The name of the server to connect to.
            
        Returns:
            The MCP client session, or None if connection failed.
            
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
            read, write = await stdio_client(server_params).__aenter__()
            session = await ClientSession(read, write).__aenter__()
            
            # Initialize the connection
            await session.initialize()
            
            return session
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
            logger.error(f"Error listing tools: {e}")
        
        return tools
    
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

async def main():
    """Main entry point for the MCP tool caller."""
    parser = argparse.ArgumentParser(description="MCP Tool Caller - A command line tool to call tools on MCP servers")
    parser.add_argument("--config", default="mcp.json", help="Path to the MCP configuration file (default: mcp.json)")
    parser.add_argument("--server", required=True, help="Name of the server to connect to")
    parser.add_argument("--tool", required=True, help="Name of the tool to call")
    parser.add_argument("--args", default="{}", help="JSON string of arguments to pass to the tool")
    parser.add_argument("--list-tools", action="store_true", help="List available tools on the server")
    
    args = parser.parse_args()
    
    try:
        tool_caller = MCPToolCaller(args.config)
        
        # Connect to the server
        session = await tool_caller.connect_to_server(args.server)
        if not session:
            print(f"Failed to connect to server: {args.server}")
            sys.exit(1)
        
        try:
            if args.list_tools:
                # List available tools
                tools = await tool_caller.list_tools(session)
                print(f"\nAvailable tools on server '{args.server}':")
                print("-" * 50)
                for tool in tools:
                    print(f"  - {tool['name']}: {tool['description']}")
                    if tool.get("input_schema") and tool["input_schema"].get("properties"):
                        print("    Arguments:")
                        for param_name, param_info in tool["input_schema"]["properties"].items():
                            required = "required" if param_name in tool["input_schema"].get("required", []) else "optional"
                            desc = param_info.get("description", "No description")
                            print(f"      {param_name} ({required}): {desc}")
            else:
                # Parse arguments
                try:
                    tool_args = json.loads(args.args)
                except json.JSONDecodeError:
                    print(f"Invalid JSON in arguments: {args.args}")
                    sys.exit(1)
                
                # Call the tool
                result = await tool_caller.call_tool(session, args.tool, tool_args)
                
                # Print the result
                print("\nTool call result:")
                print("-" * 50)
                if isinstance(result, (dict, list)):
                    print(json.dumps(result, indent=2))
                else:
                    print(result)
        finally:
            # Close the session
            await session.__aexit__(None, None, None)
    
    except FileNotFoundError:
        print(f"Configuration file not found: {args.config}")
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"Invalid JSON in configuration file: {args.config}")
        sys.exit(1)
    except ValueError as e:
        print(f"Error: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
