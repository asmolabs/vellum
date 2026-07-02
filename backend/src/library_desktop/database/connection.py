import os
from sqlalchemy import create_engine
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
    
    Base.metadata.create_all(_engine)
    _SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=_engine)

def get_session():
    if _SessionLocal is None:
        raise Exception("Database not initialized. Call init_db(library_path) first.")
    return _SessionLocal()
