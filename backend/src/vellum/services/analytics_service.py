from typing import Dict, Any
from sqlalchemy import func
from sqlalchemy.orm import Session
from ..models.book import Book, BookFormat, Tag, book_tags

class AnalyticsService:
    @staticmethod
    def get_library_stats(db: Session) -> Dict[str, Any]:
        total_books = db.query(Book).count()
        total_formats = db.query(BookFormat).count()
        
        # Calculate total storage size
        total_bytes = db.query(func.sum(BookFormat.filesize)).scalar() or 0
        size_mb = round(total_bytes / (1024 * 1024), 2)

        # Rating distribution
        ratings_raw = db.query(Book.rating, func.count(Book.id)).group_by(Book.rating).all()
        rating_dist = {f"{r[0]}⭐" if r[0] > 0 else "Non notés": r[1] for r in ratings_raw}

        # Top Authors
        authors_raw = db.query(Book.author, func.count(Book.id)).group_by(Book.author).order_by(func.count(Book.id).desc()).limit(5).all()
        top_authors = [{"author": a[0], "count": a[1]} for a in authors_raw]

        # Top Series
        series_raw = db.query(Book.series, func.count(Book.id)).filter(Book.series != None, Book.series != "").group_by(Book.series).order_by(func.count(Book.id).desc()).limit(5).all()
        top_series = [{"series": s[0], "count": s[1]} for s in series_raw]

        # Reading status
        read_count = db.query(Book).filter(Book.read_progress >= 90.0).count()
        reading_count = db.query(Book).filter(Book.read_progress > 0.0, Book.read_progress < 90.0).count()
        unread_count = total_books - (read_count + reading_count)

        return {
            "total_books": total_books,
            "total_formats": total_formats,
            "total_size_mb": size_mb,
            "read_count": read_count,
            "reading_count": reading_count,
            "unread_count": unread_count,
            "rating_distribution": rating_dist,
            "top_authors": top_authors,
            "top_series": top_series
        }
