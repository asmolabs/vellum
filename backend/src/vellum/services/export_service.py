import os
import zipfile
import tempfile
from typing import List, Dict, Optional
from ..services.books_service import BooksService

class ExportService:
    @staticmethod
    def export_books_to_zip(books_service: BooksService, book_ids: Optional[List[str]] = None) -> str:
        """Bundles requested book files into a single temporary ZIP archive."""
        if book_ids:
            books = [books_service.get_book_by_id(bid) for bid in book_ids if books_service.get_book_by_id(bid)]
        else:
            books = books_service.get_books(limit=1000)

        temp_zip = tempfile.NamedTemporaryFile(delete=False, suffix=".zip")
        temp_zip_path = temp_zip.name
        temp_zip.close()

        with zipfile.ZipFile(temp_zip_path, 'w', compression=zipfile.ZIP_DEFLATED) as zip_out:
            for book in books:
                for fmt in book.formats:
                    abs_path = os.path.join(books_service.library_path, fmt.filepath)
                    if os.path.exists(abs_path):
                        clean_author = books_service.sanitize_filename(book.author)
                        zip_entry_name = os.path.join(clean_author, fmt.filename)
                        zip_out.write(abs_path, arcname=zip_entry_name)

        return temp_zip_path

    @staticmethod
    def find_duplicates(books_service: BooksService) -> List[Dict]:
        """Scans database for books sharing identical titles or ISBNs."""
        all_books = books_service.get_books(limit=1000)
        duplicates = []
        seen_titles = {}

        for b in all_books:
            key = (b.title.lower().strip(), b.author.lower().strip())
            if key in seen_titles:
                duplicates.append({
                    "original": seen_titles[key].to_dict(),
                    "duplicate": b.to_dict(),
                    "reason": "Titre et auteur identiques"
                })
            else:
                seen_titles[key] = b

        return duplicates
