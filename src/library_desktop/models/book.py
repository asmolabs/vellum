from datetime import datetime
import uuid
from sqlalchemy import Column, String, Integer, ForeignKey, Table
from sqlalchemy.orm import relationship
from ..database.connection import Base

# Many-to-many association table for Books and Tags
book_tags = Table(
    "book_tags",
    Base.metadata,
    Column("book_id", String, ForeignKey("books.id", ondelete="CASCADE")),
    Column("tag_name", String, ForeignKey("tags.name", ondelete="CASCADE"))
)

class Tag(Base):
    __tablename__ = "tags"
    name = Column(String, primary_key=True)

class BookFormat(Base):
    __tablename__ = "book_formats"
    id = Column(String, primary_key=True, default=lambda: str(uuid.uuid4()))
    book_id = Column(String, ForeignKey("books.id", ondelete="CASCADE"), nullable=False)
    format = Column(String, nullable=False)  # 'epub', 'mobi', 'azw3'
    filename = Column(String, nullable=False)
    filepath = Column(String, nullable=False)
    filesize = Column(Integer, nullable=False)

    book = relationship("Book", back_populates="formats")

class Book(Base):
    __tablename__ = "books"
    id = Column(String, primary_key=True, default=lambda: str(uuid.uuid4()))
    title = Column(String, nullable=False)
    author = Column(String, nullable=False)
    publisher = Column(String, nullable=True)
    publishDate = Column(String, nullable=True)
    description = Column(String, nullable=True)
    coverPath = Column(String, nullable=True)
    addedAt = Column(String, default=lambda: datetime.utcnow().isoformat())

    formats = relationship("BookFormat", back_populates="book", cascade="all, delete-orphan")
    tags = relationship("Tag", secondary=book_tags, passive_deletes=True)

    def to_dict(self):
        return {
            "id": self.id,
            "title": self.title,
            "author": self.author,
            "publisher": self.publisher,
            "publishDate": self.publishDate,
            "description": self.description,
            "coverPath": self.coverPath,
            "addedAt": self.addedAt,
            "tags": [t.name for t in self.tags],
            "formats": [
                {
                    "format": f.format,
                    "filename": f.filename,
                    "filepath": f.filepath,
                    "filesize": f.filesize
                }
                for f in self.formats
            ]
        }
