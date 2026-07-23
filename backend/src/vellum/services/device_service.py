import os
import shutil
from typing import List, Dict

class DeviceService:
    @staticmethod
    def list_connected_devices() -> List[Dict[str, str]]:
        """Scans system mount points (e.g. /Volumes on macOS) for connected e-readers."""
        devices = []
        volumes_dir = "/Volumes"
        
        if not os.path.exists(volumes_dir):
            return devices

        for volume in os.listdir(volumes_dir):
            volume_path = os.path.join(volumes_dir, volume)
            if not os.path.isdir(volume_path) or volume == "Macintosh HD":
                continue

            # Check common e-reader folders
            target_folder = None
            device_type = "Liseuse USB"

            kindle_doc = os.path.join(volume_path, "documents")
            kobo_doc = os.path.join(volume_path, "books")
            root_doc = volume_path

            if os.path.exists(kindle_doc):
                target_folder = kindle_doc
                device_type = "Amazon Kindle"
            elif "kobo" in volume.lower() or os.path.exists(os.path.join(volume_path, ".kobo")):
                target_folder = volume_path
                device_type = "Kobo Reader"
            elif os.path.exists(kobo_doc):
                target_folder = kobo_doc
                device_type = "Liseuse (books)"
            else:
                # Any external USB drive
                target_folder = volume_path
                device_type = f"Support USB ({volume})"

            devices.append({
                "name": volume,
                "path": volume_path,
                "target_folder": target_folder,
                "type": device_type
            })

        return devices

    @staticmethod
    def send_to_device(source_filepath: str, target_folder: str) -> str:
        if not os.path.exists(source_filepath):
            raise Exception("Source book file not found.")
        if not os.path.exists(target_folder):
            raise Exception("Target device folder not found or disconnected.")

        filename = os.path.basename(source_filepath)
        destination_path = os.path.join(target_folder, filename)
        shutil.copyfile(source_filepath, destination_path)
        return destination_path
