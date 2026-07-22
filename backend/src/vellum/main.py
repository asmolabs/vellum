import os
import sys
import time
import socket
import threading
import subprocess
import uvicorn
import webview
from .server import app

def find_free_port():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(('', 0))
    port = s.getsockname()[1]
    s.close()
    return port

def is_port_open(port):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(('127.0.0.1', port)) == 0

class Api:
    def select_directory(self):
        window = webview.active_window()
        if window:
            result = window.create_file_dialog(webview.FOLDER_DIALOG)
            if result and len(result) > 0:
                return result[0]
        return None

def start_server(port):
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="warning")

def start_reflex(backend_dir, api_port):
    env = os.environ.copy()
    env["VELLUM_API_URL"] = f"http://127.0.0.1:{api_port}"
    # Run reflex in backend directory
    subprocess.Popen(
        [sys.executable, "-m", "reflex", "run"],
        cwd=backend_dir,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

def main():
    api_port = 8001
    if is_port_open(api_port):
        api_port = find_free_port()

    os.environ["VELLUM_API_URL"] = f"http://127.0.0.1:{api_port}"

    # Start FastAPI server in a background thread
    server_thread = threading.Thread(target=start_server, args=(api_port,), daemon=True)
    server_thread.start()

    backend_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    
    # Start Reflex server
    reflex_port = 3000
    if not is_port_open(reflex_port):
        start_reflex(backend_dir, api_port)
        # Wait up to 10 seconds for Reflex frontend server to be ready
        for _ in range(20):
            if is_port_open(reflex_port):
                break
            time.sleep(0.5)

    # Initialize pywebview window
    api = Api()
    webview.create_window(
        title="Bibliothèque Ebook",
        url=f"http://127.0.0.1:{reflex_port}",
        js_api=api,
        width=1200,
        height=800,
        min_size=(1000, 600)
    )

    # Start webview
    webview.start()

if __name__ == "__main__":
    main()
