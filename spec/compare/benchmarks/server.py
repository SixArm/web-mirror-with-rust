"""Benchmark site for run.sh: /docs/ is a tree of PAGES pages (each links to its 10
children and back to /docs/), every response delayed by LATENCY_MS to stand
in for a real network. Standard library only; threaded."""
import http.server, socketserver, sys, time

PAGES = int(sys.argv[2]) if len(sys.argv) > 2 else 200
LATENCY = (int(sys.argv[3]) if len(sys.argv) > 3 else 20) / 1000
FILLER = "<p>" + "lorem ipsum dolor sit amet " * 200 + "</p>"  # about 5 KiB

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        time.sleep(LATENCY)
        path = self.path
        if path == "/docs/":
            n = 0
        elif path.startswith("/docs/p") and path.endswith("/") and path[7:-1].isdigit():
            n = int(path[7:-1])
        else:
            self.send_response(404); self.send_header("Content-Length", "0"); self.end_headers(); return
        if n >= PAGES:
            self.send_response(404); self.send_header("Content-Length", "0"); self.end_headers(); return
        kids = "".join(f'<a href="/docs/p{c}/">p{c}</a> ' for c in range(n * 10 + 1, n * 10 + 11) if c < PAGES)
        body = f"<html><body><a href=\"/docs/\">home</a> {kids}{FILLER}</body></html>".encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

class Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True
    request_queue_size = 256

Server(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
