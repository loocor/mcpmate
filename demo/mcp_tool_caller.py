#!/usr/bin/env python3
"""
MCP Tool Caller - A command line tool to call tools on MCP servers.
"""

import asyncio
import json
import sys
from typing import Any, Dict

from mcp import ClientSession

from mcp_common import MCPClient, format_result, logger

async def call_tool(session: ClientSession, tool_name: str, arguments: Dict[str, Any]) -> Any:
    """Call a tool on the server.
    
    Args:
        session: The MCP client session.
        tool_name: The name of the tool to call.
        arguments: The arguments to pass to the tool.
        
    Returns:
        The result of the tool call.
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
    import argparse
    
    parser = argparse.ArgumentParser(description="MCP Tool Caller - A command line tool to call tools on MCP servers")
    parser.add_argument("--config", default="mcp.json", help="Path to the MCP configuration file (default: mcp.json)")
    parser.add_argument("--server", required=True, help="Name of the server to connect to")
    parser.add_argument("--tool", required=True, help="Name of the tool to call")
    parser.add_argument("--args", default="{}", help="JSON string of arguments to pass to the tool")
    parser.add_argument("--list-tools", action="store_true", help="List available tools on the server")
    
    args = parser.parse_args()
    
    try:
        client = MCPClient(args.config)
        
        # Connect to the server
        result = await client.connect_to_server(args.server)
        if not result:
            print(f"Failed to connect to server: {args.server}")
            sys.exit(1)
        
        session, stdio_ctx, session_ctx = result
        
        try:
            if args.list_tools:
                # List available tools
                tools = await client.list_tools(session)
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
                result = await call_tool(session, args.tool, tool_args)
                
                # Print the result
                print("\nTool call result:")
                print("-" * 50)
                print(format_result(result))
        finally:
            # Close the session and contexts
            try:
                await session_ctx.__aexit__(None, None, None)
                await stdio_ctx.__aexit__(None, None, None)
            except Exception as e:
                logger.warning(f"Error closing session: {e}")
    
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
