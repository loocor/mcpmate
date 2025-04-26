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

async def call_tool_sequence(session: ClientSession, tool_sequence: List[Dict[str, Any]], server_name: str = None) -> List[Any]:
    """Call a sequence of tools on the server.

    Args:
        session: The MCP client session.
        tool_sequence: A list of tool calls, each with a name and arguments.
        server_name: The default server name to use if not specified in the tool call.

    Returns:
        A list of results from each tool call.
    """
    results = []
    current_server = server_name

    for tool_call in tool_sequence:
        # Check if this tool call specifies a different server
        if "server" in tool_call:
            if tool_call["server"] != current_server:
                logger.info(f"Switching server from '{current_server}' to '{tool_call['server']}'")
                # We need to switch servers, which isn't supported in a single session
                logger.warning("Multiple servers in a single tool sequence are not supported.")
                logger.warning("Only using the first server in the sequence.")
                # We'll continue with the current server

        # Get the tool name and arguments
        if "tool" in tool_call and "arguments" in tool_call:
            # Format from sample configs: {"server": "...", "tool": "...", "arguments": {...}}
            tool_name = tool_call["tool"]
            arguments = tool_call["arguments"]
        elif "name" in tool_call and "arguments" in tool_call:
            # Alternative format: {"name": "...", "arguments": {...}}
            tool_name = tool_call["name"]
            arguments = tool_call["arguments"]
        else:
            logger.error(f"Invalid tool call format: {tool_call}")
            continue

        try:
            result = await call_tool(session, tool_name, arguments)
            results.append(result)
        except Exception as e:
            logger.error(f"Error in tool sequence at tool '{tool_name}': {e}")
            break

    return results

def parse_arguments() -> Dict[str, Any]:
    """Parse command line arguments.

    Returns:
        A dictionary of parsed arguments.
    """
    parser = argparse.ArgumentParser(description="MCP Tool - Call any MCP server tool with arguments")
    parser.add_argument("--server", help="Name of the server to connect to")
    parser.add_argument("--tool", help="Name of the tool to call")
    parser.add_argument("--args", default="{}", help="JSON string of arguments to pass to the tool")
    parser.add_argument("--conf", help="JSON file containing tool configuration or sequence")
    parser.add_argument("--output", choices=["full", "compact"], default="full",
                        help="Output format: 'full' for detailed output, 'compact' for minimal output")

    return vars(parser.parse_args())

async def main():
    """Main entry point for the MCP tool."""
    args = parse_arguments()

    server_name = args["server"]
    tool_name = args["tool"]
    tool_args_str = args["args"]
    conf_file = args["conf"]
    output_format = args["output"]

    # Check if we have a configuration file
    if conf_file:
        try:
            with open(conf_file, 'r') as f:
                conf_data = json.load(f)
        except (FileNotFoundError, json.JSONDecodeError) as e:
            print(f"Error loading configuration file: {e}")
            sys.exit(1)

        # Make sure conf_data is a list
        if not isinstance(conf_data, list):
            print(f"Error: Configuration file must contain a list of tool calls")
            sys.exit(1)

        if len(conf_data) == 0:
            print(f"Error: Configuration file contains an empty list")
            sys.exit(1)

        # Check if the configuration contains server information
        if "server" in conf_data[0]:
            # Use the server from the configuration
            server_name = conf_data[0]["server"]
            logger.info(f"Using server from configuration: {server_name}")

        # Convert the configuration to a consistent format for processing
        tool_sequence = []
        for item in conf_data:
            if "tool" in item and "arguments" in item:
                # Format from sample configs: {"server": "...", "tool": "...", "arguments": {...}}
                tool_sequence.append({
                    "name": item["tool"],
                    "arguments": item["arguments"]
                })
                if "server" in item and item["server"] != server_name:
                    logger.warning(f"Tool uses a different server: {item['server']}")
                    logger.warning("Multiple servers in a single tool sequence are not supported.")
                    logger.warning("Using the first server in the sequence.")
            else:
                logger.error(f"Invalid tool call format in configuration: {item}")

        # Update conf_data with the converted format
        conf_data = tool_sequence

    # Validate arguments
    if not tool_name and not conf_file:
        print("Error: Either --tool or --conf must be specified")
        sys.exit(1)

    if not server_name:
        print("Error: Server name must be specified either with --server or in the configuration file")
        sys.exit(1)

    # Define the callback function
    async def tool_callback(session: ClientSession) -> None:
        if tool_name and not conf_file:
            # Single tool call from command line arguments
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
            # Sequence of tool calls from configuration
            try:
                # conf_data is already loaded and converted to a consistent format above
                tool_sequence = conf_data
                logger.info(f"Processing tool sequence with {len(tool_sequence)} tool calls")
            except Exception as e:
                print(f"Error processing configuration: {e}")
                sys.exit(1)

            try:
                results = await call_tool_sequence(session, tool_sequence, server_name)

                if output_format == "full":
                    print("\nTool sequence results:")
                    print("=" * 50)
                    for i, result in enumerate(results):
                        tool_name_display = tool_sequence[i].get("name", "unknown")
                        print(f"\nResult {i+1}/{len(tool_sequence)} ({tool_name_display}):")
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
