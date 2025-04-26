#!/usr/bin/env python3
"""
MCP Tool - A generic tool for calling any MCP server tool with arguments.
"""

import asyncio
import argparse
import json
import sys
from typing import Any, Dict, List

from mcp import ClientSession

from mcp_client import run_with_server, format_result, logger

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

async def call_tool_sequence(session: ClientSession, tool_sequence: List[Dict[str, Any]]) -> List[Any]:
    """Call a sequence of tools on the server.
    
    Args:
        session: The MCP client session.
        tool_sequence: A list of tool calls, each with a name and arguments.
        
    Returns:
        A list of results from each tool call.
    """
    results = []
    
    for tool_call in tool_sequence:
        tool_name = tool_call["name"]
        arguments = tool_call["arguments"]
        
        try:
            result = await call_tool(session, tool_name, arguments)
            results.append(result)
        except Exception as e:
            logger.error(f"Error in tool sequence at tool '{tool_name}': {e}")
            # Try to clean up if needed (e.g., close browser)
            if tool_name.startswith("playwright_") and tool_name != "playwright_close":
                try:
                    await session.call_tool("playwright_close", {})
                    logger.info("Browser closed after error")
                except Exception as close_error:
                    logger.warning(f"Error closing browser: {close_error}")
            break
    
    return results

def parse_arguments() -> Dict[str, Any]:
    """Parse command line arguments.
    
    Returns:
        A dictionary of parsed arguments.
    """
    parser = argparse.ArgumentParser(description="MCP Tool - Call any MCP server tool with arguments")
    parser.add_argument("--server", required=True, help="Name of the server to connect to")
    parser.add_argument("--tool", help="Name of the tool to call")
    parser.add_argument("--args", default="{}", help="JSON string of arguments to pass to the tool")
    parser.add_argument("--sequence", help="JSON file containing a sequence of tool calls")
    parser.add_argument("--output", choices=["full", "compact"], default="full", 
                        help="Output format: 'full' for detailed output, 'compact' for minimal output")
    
    return vars(parser.parse_args())

async def main():
    """Main entry point for the MCP tool."""
    args = parse_arguments()
    
    server_name = args["server"]
    tool_name = args["tool"]
    tool_args_str = args["args"]
    sequence_file = args["sequence"]
    output_format = args["output"]
    
    if not tool_name and not sequence_file:
        print("Error: Either --tool or --sequence must be specified")
        sys.exit(1)
    
    if tool_name and sequence_file:
        print("Error: Cannot specify both --tool and --sequence")
        sys.exit(1)
    
    # Define the callback function
    async def tool_callback(session: ClientSession) -> None:
        if tool_name:
            # Single tool call
            try:
                tool_args = json.loads(tool_args_str)
            except json.JSONDecodeError:
                print(f"Error: Invalid JSON in arguments: {tool_args_str}")
                sys.exit(1)
            
            try:
                result = await call_tool(session, tool_name, tool_args)
                
                if output_format == "full":
                    print("\nTool call result:")
                    print("=" * 50)
                    print(format_result(result))
                else:
                    print(format_result(result))
            except Exception as e:
                print(f"Error calling tool: {e}")
                sys.exit(1)
        else:
            # Sequence of tool calls
            try:
                with open(sequence_file, 'r') as f:
                    tool_sequence = json.load(f)
            except (FileNotFoundError, json.JSONDecodeError) as e:
                print(f"Error loading sequence file: {e}")
                sys.exit(1)
            
            try:
                results = await call_tool_sequence(session, tool_sequence)
                
                if output_format == "full":
                    print("\nTool sequence results:")
                    print("=" * 50)
                    for i, result in enumerate(results):
                        print(f"\nResult {i+1}/{len(tool_sequence)} ({tool_sequence[i]['name']}):")
                        print("-" * 50)
                        print(format_result(result))
                else:
                    # Just print the last result
                    if results:
                        print(format_result(results[-1]))
            except Exception as e:
                print(f"Error in tool sequence: {e}")
                sys.exit(1)
    
    # Run with the specified server
    await run_with_server(server_name, tool_callback)

if __name__ == "__main__":
    asyncio.run(main())
