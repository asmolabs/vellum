import os
from sqlalchemy import create_engine, text
from sqlalchemy.orm import declarative_base, sessionmaker

Base = declarative_base()
_SessionLocal = None
_engine = None

def init_db(library_path: str):
    global _SessionLocal, _engine
    db_file = os.path.join(library_path, "metadata.db")
    # Using SQLite
    _engine = create_engine(f"sqlite:///{db_file}", connect_args={"check_same_thread": False})
    
    # Import models here to register them with Base before creating tables
    from ..models.book import Book, BookFormat, Tag
    from ..models.param import Param
    
    Base.metadata.create_all(_engine)

    # Auto-migrate newly added columns for existing databases
    with _engine.connect() as conn:
        columns_to_add = [
            ("series", "VARCHAR"),
            ("series_index", "FLOAT DEFAULT 0.0"),
            ("rating", "INTEGER DEFAULT 0"),
            ("isbn", "VARCHAR"),
            ("language", "VARCHAR DEFAULT 'fr'"),
            ("read_progress", "FLOAT DEFAULT 0.0"),
            ("last_read_at", "VARCHAR")
        ]
        for col_name, col_type in columns_to_add:
            try:
                conn.execute(text(f"ALTER TABLE books ADD COLUMN {col_name} {col_type}"))
                conn.commit()
            except Exception:
                pass  # Column already exists in table

    _SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=_engine)

def get_session():
    if _SessionLocal is None:
        raise Exception("Database not initialized. Call init_db(library_path) first.")
    return _SessionLocal()
