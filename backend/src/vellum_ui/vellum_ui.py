import sys
import os

# Ensure src directory is in sys.path
src_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if src_dir not in sys.path:
    sys.path.insert(0, src_dir)

from vellum.ui.ui import app, index

__all__ = ["app", "index"]
