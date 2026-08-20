import json
import sys


def reply(request_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()


for line in sys.stdin:
    if not line.strip():
        continue
    request = json.loads(line)
    request_id = request.get("id")
    if request_id is None:
        continue
    method = request.get("method")
    if method == "initialize":
        reply(request_id, {
            "protocolVersion": "2025-11-25",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "workflow-guide-uat", "version": "1.0.0"},
        })
    elif method == "tools/list":
        reply(request_id, {
            "tools": [{
                "name": "collect_evidence",
                "description": "Collect the concise evidence needed for the release decision.",
                "inputSchema": {"type": "object", "properties": {}},
            }],
        })
    else:
        reply(request_id, {"tools": []})
