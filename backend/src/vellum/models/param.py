import json
from datetime import datetime
from sqlalchemy import Column, String, Text, DateTime
from ..database.connection import Base

class Param(Base):
    __tablename__ = "params"

    id = Column(String, primary_key=True)  # Format: "plugin:{plugin_id}:{key}"
    plugin_id = Column(String, nullable=False, index=True)
    key = Column(String, nullable=False, default="config")
    value = Column(Text, nullable=False, default="{}")  # Stored as JSON string
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    def get_data(self) -> dict:
        try:
            return json.loads(self.value)
        except Exception:
            return {}

    def set_data(self, data: dict):
        self.value = json.dumps(data, ensure_ascii=False)
