#!/usr/bin/env python3
"""
MCP Thinking - A simple demonstration of calling the sequentialthinking tool.
"""

import asyncio
import json
import sys
from typing import Any, List

from mcp import ClientSession

from mcp_common import run_with_server, format_result, logger

async def sequential_thinking(session: ClientSession, problem: str) -> List[str]:
    """Use sequential thinking to solve a problem.
    
    Args:
        session: The MCP client session.
        problem: The problem to solve.
        
    Returns:
        A list of thoughts.
    """
    # Start with the first thought
    thought_number = 1
    total_thoughts = 5  # Initial estimate
    next_thought_needed = True
    
    print(f"\nProblem: {problem}")
    print("=" * 50)
    
    # Keep track of all thoughts
    all_thoughts = []
    
    # Continue thinking until no more thoughts are needed or max iterations reached
    max_iterations = 10
    current_iteration = 0
    
    while next_thought_needed and current_iteration < max_iterations:
        current_iteration += 1
        
        # Prepare the arguments for the current thought
        thought_text = f"Let me think about this problem: {problem}"
        if thought_number > 1 and all_thoughts:
            thought_text = f"Based on my previous thoughts, let me continue analyzing this problem."
        
        arguments = {
            "thought": thought_text,
            "thoughtNumber": thought_number,
            "totalThoughts": total_thoughts,
            "nextThoughtNeeded": True
        }
        
        logger.info(f"Calling sequentialthinking for thought {thought_number}/{total_thoughts}")
        
        try:
            result = await session.call_tool("sequentialthinking", arguments)
            
            # Extract the thought from the result
            if isinstance(result, dict):
                # Standard dictionary response
                current_thought = result.get("thought", "No thought provided")
                next_thought_needed = result.get("nextThoughtNeeded", False)
                thought_number = result.get("thoughtNumber", thought_number)
                total_thoughts = result.get("totalThoughts", total_thoughts)
                
                # Print the current thought
                print(f"\nThought {thought_number}/{total_thoughts}:")
                print("-" * 50)
                print(current_thought)
                
                # Add to all thoughts
                all_thoughts.append(current_thought)
                
                # Increment thought number for next iteration
                thought_number += 1
            elif hasattr(result, 'content') and hasattr(result.content[0], 'text'):
                # Handle the specific response format we're seeing
                try:
                    # Try to parse the JSON in the text content
                    response_json = json.loads(result.content[0].text)
                    
                    # Update our tracking variables
                    thought_number = response_json.get("thoughtNumber", thought_number)
                    total_thoughts = response_json.get("totalThoughts", total_thoughts)
                    next_thought_needed = response_json.get("nextThoughtNeeded", False)
                    
                    # For this iteration, use our original thought as the content
                    current_thought = thought_text
                    
                    # Print the current thought
                    print(f"\nThought {thought_number}/{total_thoughts}:")
                    print("-" * 50)
                    print(current_thought)
                    
                    # Add to all thoughts
                    all_thoughts.append(current_thought)
                    
                    # Increment thought number for next iteration
                    thought_number += 1
                except json.JSONDecodeError:
                    # If it's not valid JSON, just use the text as is
                    current_thought = result.content[0].text
                    
                    # Print the current thought
                    print(f"\nRaw response:")
                    print("-" * 50)
                    print(current_thought)
                    
                    # Add to all thoughts
                    all_thoughts.append(current_thought)
                    
                    # Stop after this iteration
                    next_thought_needed = False
            else:
                # Unknown format, just convert to string and print
                logger.warning(f"Unexpected result format: {result}")
                print(f"\nUnexpected response format:")
                print("-" * 50)
                print(format_result(result))
                
                # Add to all thoughts as string
                all_thoughts.append(str(result))
                
                # Stop after this iteration
                next_thought_needed = False
        except Exception as e:
            logger.error(f"Error calling sequentialthinking: {e}")
            break
    
    print("\nThinking process completed.")
    print("=" * 50)
    
    return all_thoughts

async def main():
    """Main entry point for the MCP thinking demo."""
    if len(sys.argv) < 2:
        print("Usage: python mcp_thinking.py \"<problem to solve>\"")
        sys.exit(1)
    
    problem = sys.argv[1]
    
    # Define the callback function
    async def thinking_callback(session: ClientSession) -> None:
        all_thoughts = await sequential_thinking(session, problem)
        
        if all_thoughts:
            print("\nAll thoughts:")
            for i, thought in enumerate(all_thoughts, 1):
                print(f"\n{i}. {thought}")
        else:
            print("No thoughts were generated.")
    
    # Run with the thinking server
    await run_with_server("thinking", thinking_callback)

if __name__ == "__main__":
    asyncio.run(main())
