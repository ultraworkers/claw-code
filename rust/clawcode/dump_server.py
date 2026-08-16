import json, sys, time, uuid, os
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse

LOG_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dump_output.txt")

def log(msg):
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(msg + "\n")
        f.flush()

class DumpHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length)
        path = urlparse(self.path).path

        log(f"\n{'='*70}")
        log(f"REQUEST: POST {path}")
        log(f"HEADERS: {json.dumps(dict(self.headers))}")
        log(f"BODY SIZE: {len(body)} bytes")
        try:
            parsed = json.loads(body)
            sys_prompt = ""
            raw_system = parsed.get("system")
            if raw_system is not None:
                if isinstance(raw_system, str):
                    sys_prompt = raw_system
                elif isinstance(raw_system, list):
                    parts = []
                    for block in raw_system:
                        if isinstance(block, dict):
                            text = block.get("text", "") or ""
                            parts.append(text)
                    sys_prompt = "\n".join(parts)
            if not sys_prompt:
                for msg in parsed.get("messages", []):
                    if msg.get("role") == "system":
                        c = msg.get("content", "")
                        sys_prompt = c if isinstance(c, str) else str(c)
                        break
            system_chars = len(sys_prompt)
            system_tokens = system_chars // 4

            tools = parsed.get("tools", [])
            all_messages = parsed.get("messages", [])
            non_sys_msgs = [m for m in all_messages if m.get("role") != "system"]

            msg_chars = sum(len(json.dumps(m, ensure_ascii=False)) for m in non_sys_msgs) if non_sys_msgs else 0
            tool_chars = sum(len(json.dumps(t, ensure_ascii=False)) for t in tools) if tools else 0

            log(f"\n=== SIZE BREAKDOWN ===")
            log(f"System prompt:    {system_chars:>6} chars / ~{system_tokens:>5} tokens")
            log(f"Messages ({len(non_sys_msgs)}):   {msg_chars:>6} bytes")
            log(f"Tools ({len(tools)}):       {tool_chars:>6} bytes")
            log(f"Total body:       {len(body):>6} bytes")
            log(f'Model: {parsed.get("model", "N/A")}')
            log(f'Stream: {parsed.get("stream", "N/A")}')
            log(f'Max tokens: {parsed.get("max_tokens", parsed.get("max_completion_tokens", "N/A"))}')

            if sys_prompt:
                log(f"\n=== SYSTEM PROMPT (full) ===")
                log(sys_prompt)

            if tools:
                log(f"\n=== TOOLS ({len(tools)}) ===")
                for t in tools:
                    fname = t.get("name") or t.get("function", {}).get("name", "?")
                    fdesc = t.get("description") or t.get("function", {}).get("description", "")
                    log(f"  - {fname}: {fdesc}")

            if non_sys_msgs:
                log(f"\n=== MESSAGES ===")
                for m in non_sys_msgs:
                    role = m.get("role", "?")
                    c = m.get("content", "")
                    if isinstance(c, list):
                        parts = [p.get("type","?")[:20] for p in c if isinstance(p,dict)]
                        content_str = f"[{'|'.join(parts)}]"
                    else:
                        content_str = str(c)
                    log(f"  [{role}]: {content_str}")

            log(f"\n=== FULL JSON BODY (pretty) ===")
            pretty = json.dumps(parsed, indent=2, ensure_ascii=False)
            log(pretty)
            log(f"  skip_tools: {parsed.get('tools') is None}")
        except Exception as e:
            log(f"\nPARSE ERROR: {e}")
            import traceback
            traceback.print_exc(file=open(LOG_FILE, "a"))
            log(f"  skip_tools: True (unparseable)")

        # Send Anthropic-compatible SSE (/v1/messages format)
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()

        msg_id = str(uuid.uuid4())

        # Always return a single end_turn text response; no tool_use round trip.
        events = [
            {"type": "message_start", "message": {"id": msg_id, "type": "message", "role": "assistant", "content": [], "model": "local-model", "stop_reason": None, "stop_sequence": None, "usage": {"input_tokens": 10, "output_tokens": 5}}},
            {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
            {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Request received."}},
            {"type": "content_block_stop", "index": 0},
            {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": 5}},
            {"type": "message_stop"},
        ]
        for evt in events:
            self.wfile.write(f"data: {json.dumps(evt)}\n\n".encode())
            self.wfile.flush()
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()
        time.sleep(0.1)

    def log_message(self, format, *args):
        pass

port = 1234
log(f"Dump server starting on port {port}")
server = HTTPServer(("0.0.0.0", port), DumpHandler)
server.serve_forever()
