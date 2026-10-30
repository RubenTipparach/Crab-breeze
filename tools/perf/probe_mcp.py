"""An MCP server (stdio, JSON-RPC 2.0, no dependencies) in front of a running game's perf probe (openspec change
`perf-probe`, `crates/cb-app/src/perf_probe.rs`): a session asks a game built with `--features perf_probe` what its
last frames cost, what took the time in the slow ones and what happened in the game then, while it plays.

    claude mcp add crab-perf -- python tools/perf/probe_mcp.py
    (or in .mcp.json: {"mcpServers": {"crab-perf": {"command": "python", "args": ["tools/perf/probe_mcp.py"]}}})

The probe's address is CB_PROBE_URL (default http://127.0.0.1:7878). Tools:
- perf_summary(seconds): frames, p50/p95/p99/max, frames over 16.7 and 11.1 ms, the ten worst with their systems,
  counts and game events, the systems most often to blame, and the world's census.
- perf_events(since): game events (blasts, falling buildings, debris) after a frame, each with its frame's ms.
- perf_frames(since, limit): per-frame [n, t_s, ms, main_ms, x, y, z, speed].
- perf_census(): entities by name, physics bodies by kind, colliders, meshes drawn and in view, lights.
"""

import json
import os
import sys
import urllib.error
import urllib.request

URL = os.environ.get("CB_PROBE_URL", "http://127.0.0.1:7878")

TOOLS = [
    {
        "name": "perf_summary",
        "description": "The running game's last N seconds: frame percentiles, slow frames with the systems and game "
        "events in them, the systems most often to blame, and the world's census.",
        "inputSchema": {"type": "object", "properties": {"seconds": {"type": "number", "default": 10}}},
    },
    {
        "name": "perf_events",
        "description": "Game events (blasts, buildings falling, debris spawned) after frame `since`, each with its "
        "frame's and the next frame's milliseconds.",
        "inputSchema": {"type": "object", "properties": {"since": {"type": "integer", "default": 0}}},
    },
    {
        "name": "perf_frames",
        "description": "Per-frame rows [n, t_s, ms, main_ms, x, y, z, speed_m_s] after frame `since`.",
        "inputSchema": {
            "type": "object",
            "properties": {"since": {"type": "integer", "default": 0}, "limit": {"type": "integer", "default": 500}},
        },
    },
    {
        "name": "perf_census",
        "description": "What the world holds now: entities by name, physics bodies by kind, colliders, meshes drawn "
        "and in view, lights.",
        "inputSchema": {"type": "object", "properties": {}},
    },
]


def probe(path):
    try:
        with urllib.request.urlopen(URL + path, timeout=5) as r:
            return r.read().decode("utf-8")
    except (urllib.error.URLError, OSError) as e:
        return json.dumps({"error": f"no game probe at {URL}: {e}. Run a build with --features perf_probe."})


def call(name, args):
    if name == "perf_summary":
        return probe(f"/summary?s={float(args.get('seconds', 10))}")
    if name == "perf_events":
        return probe(f"/events?since={int(args.get('since', 0))}")
    if name == "perf_frames":
        return probe(f"/frames?since={int(args.get('since', 0))}&limit={int(args.get('limit', 500))}")
    if name == "perf_census":
        return probe("/census")
    raise KeyError(name)


def reply(id_, result=None, error=None):
    msg = {"jsonrpc": "2.0", "id": id_}
    if error is not None:
        msg["error"] = error
    else:
        msg["result"] = result
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def main():
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue
        method, id_ = req.get("method"), req.get("id")
        if id_ is None:
            continue  # a notification (notifications/initialized and the like)
        if method == "initialize":
            reply(
                id_,
                {
                    "protocolVersion": req.get("params", {}).get("protocolVersion", "2025-06-18"),
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "crab-perf", "version": "1.0.0"},
                },
            )
        elif method == "tools/list":
            reply(id_, {"tools": TOOLS})
        elif method == "tools/call":
            p = req.get("params", {})
            try:
                text = call(p.get("name"), p.get("arguments") or {})
                reply(id_, {"content": [{"type": "text", "text": text}]})
            except KeyError:
                reply(id_, error={"code": -32602, "message": f"unknown tool {p.get('name')}"})
        elif method == "ping":
            reply(id_, {})
        else:
            reply(id_, error={"code": -32601, "message": f"no method {method}"})


if __name__ == "__main__":
    main()
