#!/usr/bin/env python3
"""
MCP Scrape - A simple demonstration of calling the firecrawl_scrape tool.
"""

import asyncio
import sys
from typing import Any

from mcp import ClientSession

from mcp_common import run_with_server, format_result, logger

async def scrape_url(session: ClientSession, url: str) -> Any:
    """Scrape a URL using the firecrawl_scrape tool.
    
    Args:
        session: The MCP client session.
        url: The URL to scrape.
        
    Returns:
        The scrape result.
    """
    # Prepare arguments for the firecrawl_scrape tool
    arguments = {
        "url": url,
        "formats": ["markdown"],
        "onlyMainContent": True
    }
    
    # Call the tool
    logger.info(f"Scraping URL: {url}")
    result = await session.call_tool("firecrawl_scrape", arguments)
    
    return result

async def main():
    """Main entry point for the MCP scrape demo."""
    if len(sys.argv) < 2:
        print("Usage: python mcp_scrape.py <url>")
        sys.exit(1)
    
    url = sys.argv[1]
    
    # Define the callback function
    async def scrape_callback(session: ClientSession) -> None:
        result = await scrape_url(session, url)
        
        print("\nScrape result:")
        print("=" * 50)
        
        if isinstance(result, dict):
            for format_name, content in result.items():
                print(f"\nFormat: {format_name}")
                print("-" * 50)
                if isinstance(content, str):
                    # Limit output to first 1000 characters
                    if len(content) > 1000:
                        print(content[:1000] + "...\n[Content truncated]")
                    else:
                        print(content)
                else:
                    print(format_result(content))
        else:
            print(format_result(result))
    
    # Run with the firecrawl server
    await run_with_server("firecrawl", scrape_callback)

if __name__ == "__main__":
    asyncio.run(main())
