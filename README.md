# MCP Client

A minimal command line client for MCP servers.

## Features

- Connect to MCP servers
- List all available resources
- List all available tools
- List all available prompts
- Display detailed information about the server

## Installation

### Using uv (Recommended)

```bash
# Create a virtual environment
uv venv

# Activate the virtual environment
source .venv/bin/activate  # On Linux/macOS
# or
.venv\Scripts\activate  # On Windows

# Install dependencies
uv pip install mcp
```

### Using pip

```bash
pip install -e .
```

## Usage

### Basic Usage

```bash
python mcp_client.py
```

This will list all servers in the configuration file and prompt you to select one to connect to.

### Command Line Options

- `--config <path>`: Specify the configuration file path (default: mcp.json)
- `--server <name>`: Specify the server to connect to
- `--list`: List all servers in the configuration file
- `--all`: Connect to all servers in the configuration file

### Examples

List all servers:

```bash
python mcp_client.py --list
```

Connect to a specific server:

```bash
python mcp_client.py --server firecrawl
```

Connect to all servers:

```bash
python mcp_client.py --all
```

## Configuration File

The configuration file uses JSON format, compatible with Claude Desktop:

```json
{
  "mcpServers": {
    "server_name": {
      "command": "command_to_run",
      "args": ["arg1", "arg2", ...],
      "env": {
        "ENV_VAR1": "value1",
        "ENV_VAR2": "value2"
      }
    },
    ...
  }
}
```

- `server_name`: Name of the server
- `command`: Command to start the server
- `args`: Command line arguments (optional)
- `env`: Environment variables (optional)
