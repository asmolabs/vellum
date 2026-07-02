import socket
import threading
import uvicorn
import webview
from .server import app

def find_free_port():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(('', 0))
    port = s.getsockname()[1]
    s.close()
    return port

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

def main():
    port = find_free_port()
    
    # Start FastAPI server in a background thread
    server_thread = threading.Thread(target=start_server, args=(port,), daemon=True)
    server_thread.start()
    
    # Initialize pywebview window
    api = Api()
    webview.create_window(
        title="Bibliothèque Ebook",
        url=f"http://127.0.0.1:{port}",
        js_api=api,
        width=1200,
        height=800,
        min_size=(1000, 600)
      )
    
    # Start webview
    webview.start()

if __name__ == "__main__":
    main()
