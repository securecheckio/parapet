#!/usr/bin/env python3
"""
Simple HTTPS reverse proxy for local RPC testing
Proxies HTTPS requests on port 8443 to HTTP on port 8899
"""
import ssl
import subprocess
import os
from http.server import HTTPServer, BaseHTTPRequestHandler
import urllib.request
import json

# Configuration
HTTPS_PORT = 9443
BACKEND_URL = "http://localhost:8899"
CERT_FILE = "/tmp/rpc-ssl/cert.pem"
KEY_FILE = "/tmp/rpc-ssl/key.pem"

class ProxyHandler(BaseHTTPRequestHandler):
    def do_OPTIONS(self):
        print(f"🔒 OPTIONS {self.path} from {self.client_address[0]}")
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', '*')
        self.send_header('Access-Control-Allow-Headers', '*')
        self.send_header('Access-Control-Max-Age', '86400')
        self.end_headers()
    
    def do_POST(self):
        # Read request body
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length)
        
        # Parse and log RPC method
        try:
            rpc_data = json.loads(body)
            method = rpc_data.get('method', 'unknown')
            print(f"🔒 POST {self.path} from {self.client_address[0]} - method: {method}")
        except:
            print(f"🔒 POST {self.path} from {self.client_address[0]}")
        
        # Forward to backend with path preserved
        backend_url = BACKEND_URL + self.path
        try:
            req = urllib.request.Request(
                backend_url,
                data=body,
                headers={'Content-Type': 'application/json'}
            )
            
            with urllib.request.urlopen(req) as response:
                response_body = response.read()
                
            # Send response
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Expose-Headers', '*')
            self.end_headers()
            self.wfile.write(response_body)
            print(f"   ✅ Response sent ({len(response_body)} bytes)")
            
        except urllib.error.HTTPError as e:
            print(f"   ❌ Upstream error: {e.code} - {e.reason}")
            self.send_error(e.code, f"Upstream error: {str(e)}")
        except Exception as e:
            print(f"   ❌ Proxy error: {str(e)}")
            self.send_error(500, f"Proxy error: {str(e)}")
    
    def do_GET(self):
        print(f"🔒 GET {self.path} from {self.client_address[0]}")
        # Forward GET requests with path preserved
        backend_url = BACKEND_URL + self.path
        try:
            with urllib.request.urlopen(backend_url) as response:
                response_body = response.read()
                
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(response_body)
            print(f"   ✅ Response sent ({len(response_body)} bytes)")
            
        except urllib.error.HTTPError as e:
            print(f"   ❌ Upstream error: {e.code} - {e.reason}")
            self.send_error(e.code, f"Upstream error: {str(e)}")
        except Exception as e:
            print(f"   ❌ Proxy error: {str(e)}")
            self.send_error(500, f"Proxy error: {str(e)}")
    
    def log_message(self, format, *args):
        # Custom logging
        print(f"🔒 HTTPS Request: {format % args}")

def generate_certificate():
    """Generate self-signed certificate if it doesn't exist"""
    os.makedirs("/tmp/rpc-ssl", exist_ok=True)
    
    if os.path.exists(CERT_FILE) and os.path.exists(KEY_FILE):
        print("✅ Using existing certificate")
        return
    
    print("📜 Generating self-signed certificate...")
    subprocess.run([
        "openssl", "req", "-x509", "-newkey", "rsa:2048",
        "-keyout", KEY_FILE, "-out", CERT_FILE,
        "-days", "365", "-nodes",
        "-subj", "/CN=192.168.86.37"
    ], check=True, capture_output=True)
    print("✅ Certificate generated!")

def main():
    print("🔐 Starting HTTPS Proxy for RPC...")
    print()
    
    # Generate certificate
    generate_certificate()
    
    # Create HTTPS server
    httpd = HTTPServer(('0.0.0.0', HTTPS_PORT), ProxyHandler)
    
    # Wrap with SSL
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(CERT_FILE, KEY_FILE)
    httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
    
    print()
    print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
    print("✅ HTTPS Proxy is running!")
    print()
    print("📱 Use this URL in Backpack mobile:")
    print("   https://192.168.86.37:9443")
    print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
    print()
    print("⚠️  Your phone will show a certificate warning.")
    print("   This is normal for self-signed certificates.")
    print("   Tap 'Advanced' → 'Proceed anyway'")
    print()
    print("Press Ctrl+C to stop")
    print()
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n👋 Shutting down HTTPS proxy...")
        httpd.shutdown()

if __name__ == "__main__":
    main()
