#!/usr/bin/env python3
"""ACP header-delivery probe (`openspec/changes/mcp-auth`, task 4.1; design.md D3).

Usage: scripts/acp-header-probe.py <label> <adapter command> [args...]

Re-run it for each adapter when the adapter is updated, and update the
per-adapter table in design.md. It uses a dummy token only - never a real
credential. Set PROBE_GET_401=1 to answer a header-less GET with 401.

Starts a loopback MCP stub, asks the adapter for a session whose mcpServers
entry carries a dummy bearer token, and reports (booleans only) whether the
stub saw the header and whether the token appears in any descendant's argv."""
import http.server, json, os, select, signal, subprocess, sys, threading, time
TOKEN = "knotprobe_dummy_7c1f0e"
label, cmd = sys.argv[1], sys.argv[2:]
hits = []
class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def _reply(self, code, body=None):
        self.send_response(code)
        if body is not None:
            data = json.dumps(body).encode()
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers(); self.wfile.write(data)
        else:
            self.send_header("Content-Length", "0"); self.end_headers()
    def do_GET(self):
        ok = self.headers.get("Authorization") == f"Bearer {TOKEN}"
        hits.append(("GET", ok))
        # PROBE_GET_401=1 answers a header-less stream GET as Knot's server
        # does, to check the adapter keeps working on POST alone.
        self._reply(401 if os.environ.get("PROBE_GET_401") == "1" and not ok else 405)
    def do_POST(self):
        n = int(self.headers.get("Content-Length") or 0)
        try: msg = json.loads(self.rfile.read(n) or b"{}")
        except Exception: msg = {}
        hits.append((msg.get("method", "?") if isinstance(msg, dict) else "batch",
                     self.headers.get("Authorization") == f"Bearer {TOKEN}"))
        if isinstance(msg, dict) and "id" in msg:
            m = msg.get("method")
            res = {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}},
                   "serverInfo": {"name": "probe", "version": "0"}} if m == "initialize" else {"tools": []}
            self._reply(200, {"jsonrpc": "2.0", "id": msg["id"], "result": res})
        else:
            self._reply(202)
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
port = srv.server_address[1]
threading.Thread(target=srv.serve_forever, daemon=True).start()
p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                     cwd="/tmp/acp-probe-cwd", start_new_session=True)
buf = b""
def send(o): p.stdin.write((json.dumps(o) + "\n").encode()); p.stdin.flush()
def wait_id(i, t):
    global buf
    end = time.time() + t
    while time.time() < end:
        r, _, _ = select.select([p.stdout], [], [], 0.5)
        if not r: continue
        chunk = os.read(p.stdout.fileno(), 65536)
        if not chunk: return None
        buf += chunk
        while b"\n" in buf:
            line, buf = buf.split(b"\n", 1)
            try: m = json.loads(line)
            except Exception: continue
            if m.get("id") == i and ("result" in m or "error" in m): return m
    return None
def descendants(root):
    rows = subprocess.run(["ps", "-A", "-o", "pid=,ppid="], capture_output=True, text=True).stdout.split("\n")
    kids = {}
    for r in rows:
        f = r.split()
        if len(f) == 2: kids.setdefault(f[1], []).append(f[0])
    out, stack = [], [str(root)]
    while stack:
        x = stack.pop(); out.append(x); stack.extend(kids.get(x, []))
    return out
def token_in_argv():
    out = descendants(p.pid)
    found = []
    res["mcp_config_flag_in"] = []
    for pid in out:
        a = subprocess.run(["ps", "-o", "args=", "-p", pid], capture_output=True, text=True).stdout
        c = subprocess.run(["ps", "-o", "comm=", "-p", pid], capture_output=True, text=True).stdout.strip()
        if TOKEN in a: found.append(os.path.basename(c))
        if "--mcp-config" in a: res["mcp_config_flag_in"].append(os.path.basename(c))
    return len(out), found
res = {"adapter": label}
try:
    send({"jsonrpc": "2.0", "id": 1, "method": "initialize",
          "params": {"protocolVersion": 1, "clientCapabilities": {}}})
    init = wait_id(1, 60)
    caps = ((init or {}).get("result") or {}).get("agentCapabilities", {}).get("mcpCapabilities", {})
    res["init_ok"] = bool(init and "result" in init); res["mcp_http"] = bool(caps.get("http"))
    send({"jsonrpc": "2.0", "id": 2, "method": "session/new",
          "params": {"cwd": "/tmp/acp-probe-cwd", "mcpServers": [{"type": "http", "name": "knotprobe",
                     "url": f"http://127.0.0.1:{port}/mcp",
                     "headers": [{"name": "Authorization", "value": f"Bearer {TOKEN}"}]}]}})
    sn = wait_id(2, 90)
    res["session_new"] = "ok" if sn and "result" in sn else ("error: " + str((sn or {}).get("error", {}).get("message", "timeout"))[:120])
    time.sleep(12)
    res["procs"], res["token_in_argv_of"] = token_in_argv()
    res["stub_requests"] = len(hits)
    res["requests_with_header"] = sum(1 for _, ok in hits if ok)
    res["methods"] = sorted({m for m, _ in hits})
    res["methods_without_header"] = sorted({m for m, ok in hits if not ok})
finally:
    for pid in reversed(descendants(p.pid)):
        try: os.kill(int(pid), signal.SIGTERM)
        except Exception: pass
    srv.shutdown()
print(json.dumps(res))
