#!/usr/bin/env python3
"""
MCP Info - A simple demonstration of connecting to MCP servers and retrieving information.
"""

import asyncio
import sys
from typing import Any, Dict

from mcp_common import MCPClient, logger

async def main():
    """Main entry point for the MCP info demo."""
    # Create MCP client
    client = MCPClient()
    
    # Get server names
    server_names = client.get_server_names()
    if not server_names:
        logger.error("No servers found in configuration")
        sys.exit(1)
    
    print(f"Found {len(server_names)} servers in configuration: {', '.join(server_names)}")
    
    # Connect to each server and get information
    for server_name in server_names:
        # Connect to the server
        result = await client.connect_to_server(server_name)
        if not result:
            logger.error(f"Failed to connect to server: {server_name}")
            continue
        
        session, stdio_ctx, session_ctx = result
        
        try:
            # Get server information
            server_info = await client.get_server_info(session)
            
            # Print server information
            client.print_server_info(server_name, server_info)
        finally:
            # Close the session and contexts
            try:
                await session_ctx.__aexit__(None, None, None)
                await stdio_ctx.__aexit__(None, None, None)
            except Exception as e:
                logger.warning(f"Error closing session for server '{server_name}': {e}")

if __name__ == "__main__":
    asyncio.run(main())
