#!/usr/bin/env python3
"""
MCP Playwright - A simple demonstration of calling the playwright_navigate and playwright_screenshot tools.
"""

import asyncio
import base64
import os
import sys
from typing import Any

from mcp import ClientSession

from mcp_common import run_with_server, format_result, logger

async def navigate_and_screenshot(session: ClientSession, url: str) -> Any:
    """Navigate to a URL and take a screenshot.
    
    Args:
        session: The MCP client session.
        url: The URL to navigate to.
        
    Returns:
        The screenshot result.
    """
    # First, navigate to the URL
    navigate_args = {
        "url": url,
        "width": 1280,
        "height": 800,
        "headless": True
    }
    
    logger.info(f"Navigating to URL: {url}")
    
    try:
        navigate_result = await session.call_tool("playwright_navigate", navigate_args)
        logger.info("Navigation successful")
        
        # Now take a screenshot
        screenshot_args = {
            "name": "screenshot",
            "fullPage": True,
            "storeBase64": True
        }
        
        logger.info("Taking screenshot")
        screenshot_result = await session.call_tool("playwright_screenshot", screenshot_args)
        
        # Close the browser
        logger.info("Closing browser")
        await session.call_tool("playwright_close", {})
        
        return screenshot_result
    except Exception as e:
        logger.error(f"Error in navigate_and_screenshot: {e}")
        
        # Try to close the browser even if there was an error
        try:
            await session.call_tool("playwright_close", {})
        except Exception as close_error:
            logger.warning(f"Error closing browser: {close_error}")
        
        raise

async def main():
    """Main entry point for the MCP Playwright demo."""
    if len(sys.argv) < 2:
        print("Usage: python mcp_playwright.py <url>")
        sys.exit(1)
    
    url = sys.argv[1]
    
    # Define the callback function
    async def playwright_callback(session: ClientSession) -> None:
        screenshot_result = await navigate_and_screenshot(session, url)
        
        print("\nScreenshot taken successfully!")
        print("=" * 50)
        
        # Handle different response formats
        if isinstance(screenshot_result, dict) and "base64" in screenshot_result:
            # Standard dictionary response
            print(f"Screenshot size: {len(screenshot_result['base64'])} bytes")
            print("Screenshot is available in base64 format")
            
            # Save the screenshot to a file if it's in base64 format
            if screenshot_result.get("base64"):
                # Create screenshots directory if it doesn't exist
                os.makedirs("screenshots", exist_ok=True)
                
                # Save the screenshot
                screenshot_path = os.path.join("screenshots", "screenshot.png")
                with open(screenshot_path, "wb") as f:
                    f.write(base64.b64decode(screenshot_result["base64"]))
                
                print(f"Screenshot saved to: {screenshot_path}")
        elif hasattr(screenshot_result, 'content') and hasattr(screenshot_result.content[0], 'text'):
            # Handle the specific response format we're seeing
            print("Screenshot result contains text content")
            print(screenshot_result.content[0].text)
            
            # Try to extract the screenshot path from the text
            text = screenshot_result.content[0].text
            if "Screenshot saved to:" in text:
                print(f"Screenshot saved to: {text.split('Screenshot saved to:')[1].strip()}")
        else:
            # Unknown format, just convert to string
            print("Screenshot result (string representation):")
            print(format_result(screenshot_result))
    
    # Run with the playwright server
    await run_with_server("playwright", playwright_callback)

if __name__ == "__main__":
    asyncio.run(main())
