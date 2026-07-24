import os
import shutil
import uuid
from typing import List, Optional
from sqlalchemy import func
from sqlalchemy.orm import Session
from ..models.book import Book, BookFormat, Tag
from .metadata_service import parse_metadata
from .plugins_service import PluginsService

class BooksService:
    def __init__(self, db: Session, plugins_service: PluginsService, library_path: str):
        self.db = db
        self.plugins_service = plugins_service
        self.library_path = library_path

    def sanitize_filename(self, name: str) -> str:
        return "".join(c for c in name if c not in r'\/:*?"<>|').strip()

    def get_books(self, search: Optional[str] = None, tag: Optional[str] = None, skip: int = 0, limit: int = 20) -> List[Book]:
        query = self.db.query(Book)
        if search:
            q = f"%{search.lower()}%"
            query = query.filter(
                Book.title.ilike(q) | Book.author.ilike(q) | Book.description.ilike(q) | Book.series.ilike(q) | Book.isbn.ilike(q)
            )
        if tag:
            query = query.join(Book.tags).filter(Tag.name == tag)
        
        return query.order_by(Book.addedAt.desc()).offset(skip).limit(limit).all()

    def get_tags_with_counts(self) -> dict:
        from ..models.book import book_tags
        results = self.db.query(
            book_tags.c.tag_name, 
            func.count(book_tags.c.book_id)
        ).group_by(book_tags.c.tag_name).all()
        
        total_books = self.db.query(Book).count()
        return {
            "total": total_books,
            "tags": {r[0]: r[1] for r in results}
        }

    def get_book_by_id(self, book_id: str) -> Optional[Book]:
        return self.db.query(Book).filter(Book.id == book_id).first()

    def get_book_file(self, book_id: str, format_name: str) -> Optional[BookFormat]:
        return self.db.query(BookFormat).filter(
            BookFormat.book_id == book_id, 
            BookFormat.format == format_name.lower()
        ).first()

    async def import_book(self, upload_file) -> Book:
        temp_dir = os.path.join(self.library_path, ".temp")
        os.makedirs(temp_dir, exist_ok=True)
        temp_file_path = os.path.join(temp_dir, f"{uuid.uuid4()}-{upload_file.filename}")
        
        try:
            with open(temp_file_path, "wb") as buffer:
                shutil.copyfileobj(upload_file.file, buffer)
                
            ext = os.path.splitext(upload_file.filename)[1].lower().replace(".", "")
            if ext not in ["epub", "mobi", "azw3"]:
                raise Exception("Unsupported file format")
                
            meta = parse_metadata(temp_file_path)
            
            enriched_meta = self.plugins_service.run_import_plugins(temp_file_path, {
                "title": meta.title or "Untitled",
                "author": meta.author or "Unknown",
                "tags": meta.tags or [],
                "description": meta.description
            })
            
            clean_author = self.sanitize_filename(enriched_meta.get("author") or "Unknown")
            clean_title = self.sanitize_filename(enriched_meta.get("title") or "Untitled")
            book_id = str(uuid.uuid4())
            
            book_dir_rel = os.path.join(clean_author, f"{clean_title} ({book_id})")
            book_dir_abs = os.path.join(self.library_path, book_dir_rel)
            os.makedirs(book_dir_abs, exist_ok=True)
            
            filename = f"{clean_title} ({book_id}).{ext}"
            dest_filepath_rel = os.path.join(book_dir_rel, filename)
            dest_filepath_abs = os.path.join(book_dir_abs, filename)
            
            shutil.copyfile(temp_file_path, dest_filepath_abs)
            filesize = os.path.getsize(dest_filepath_abs)
            
            cover_path_rel = None
            cover_data = meta.cover_data
            cover_ext = meta.cover_ext or ".jpg"
            if cover_data:
                cover_filename = f"cover{cover_ext}"
                cover_path_abs = os.path.join(book_dir_abs, cover_filename)
                with open(cover_path_abs, "wb") as f:
                    f.write(cover_data)
                cover_path_rel = os.path.join(book_dir_rel, cover_filename)
                
            new_book = Book(
                id=book_id,
                title=enriched_meta["title"],
                author=enriched_meta["author"],
                publisher=meta.publisher,
                publishDate=meta.publish_date,
                description=enriched_meta.get("description"),
                coverPath=cover_path_rel
            )
            
            for tag_name in enriched_meta.get("tags", []):
                tag = self.db.query(Tag).filter(Tag.name == tag_name).first()
                if not tag:
                    tag = Tag(name=tag_name)
                    self.db.add(tag)
                new_book.tags.append(tag)
                
            fmt = BookFormat(
                book_id=book_id,
                format=ext,
                filename=filename,
                filepath=dest_filepath_rel,
                filesize=filesize
            )
            new_book.formats.append(fmt)
            
            self.db.add(new_book)
            self.db.commit()
            self.db.refresh(new_book)
            
            return new_book
        finally:
            if os.path.exists(temp_file_path):
                try:
                    os.remove(temp_file_path)
                except:
                    pass

    def update_book(
        self, 
        book_id: str, 
        title: str, 
        author: str, 
        publisher: Optional[str], 
        publish_date: Optional[str], 
        description: Optional[str], 
        tags: List[str],
        series: Optional[str] = None,
        series_index: Optional[float] = 0.0,
        rating: Optional[int] = 0,
        isbn: Optional[str] = None,
        language: Optional[str] = "fr"
    ) -> Book:
        book = self.get_book_by_id(book_id)
        if not book:
            raise Exception("Book not found")
            
        old_author = book.author
        old_title = book.title
        
        book.title = title
        book.author = author
        book.publisher = publisher
        book.publishDate = publish_date
        book.description = description
        book.series = series
        book.series_index = series_index
        book.rating = rating
        book.isbn = isbn
        book.language = language
        
        book.tags.clear()
        for tag_name in tags:
            tag = self.db.query(Tag).filter(Tag.name == tag_name).first()
            if not tag:
                tag = Tag(name=tag_name)
                self.db.add(tag)
            book.tags.append(tag)

    def update_reading_progress(self, book_id: str, progress: float) -> Book:
        from datetime import datetime
        book = self.get_book_by_id(book_id)
        if not book:
            raise Exception("Book not found")
        book.read_progress = max(0.0, min(100.0, progress))
        book.last_read_at = datetime.utcnow().isoformat()
        self.db.commit()
        self.db.refresh(book)
        return book

    def bulk_update_books(
        self,
        book_ids: List[str],
        author: Optional[str] = None,
        publisher: Optional[str] = None,
        series: Optional[str] = None,
        rating: Optional[int] = None,
        add_tags: List[str] = []
    ) -> int:
        count = 0
        for b_id in book_ids:
            book = self.get_book_by_id(b_id)
            if not book:
                continue
            if author:
                book.author = author
            if publisher:
                book.publisher = publisher
            if series:
                book.series = series
            if rating is not None and rating >= 0:
                book.rating = rating
            for t_name in add_tags:
                tag = self.db.query(Tag).filter(Tag.name == t_name).first()
                if not tag:
                    tag = Tag(name=t_name)
                    self.db.add(tag)
                if tag not in book.tags:
                    book.tags.append(tag)
            count += 1
        self.db.commit()
        return count

    def bulk_delete_books(self, book_ids: List[str]) -> int:
        count = 0
        for b_id in book_ids:
            try:
                self.delete_book(b_id)
                count += 1
            except Exception:
                pass
        return count
            
        if old_author != author or old_title != title:
            if book.formats:
                first_fmt = book.formats[0]
                old_first_filepath_abs = os.path.join(self.library_path, first_fmt.filepath)
                old_book_dir_abs = os.path.dirname(old_first_filepath_abs)
                
                if os.path.exists(old_book_dir_abs):
                    clean_author = self.sanitize_filename(author)
                    clean_title = self.sanitize_filename(title)
                    
                    new_book_dir_rel = os.path.join(clean_author, f"{clean_title} ({book.id})")
                    new_book_dir_abs = os.path.join(self.library_path, new_book_dir_rel)
                    
                    os.makedirs(os.path.dirname(new_book_dir_abs), exist_ok=True)
                    
                    try:
                        shutil.move(old_book_dir_abs, new_book_dir_abs)
                        
                        old_author_dir_abs = os.path.dirname(old_book_dir_abs)
                        if os.path.exists(old_author_dir_abs) and not os.listdir(old_author_dir_abs):
                            os.rmdir(old_author_dir_abs)
                            
                        for fmt in book.formats:
                            file_ext = os.path.splitext(fmt.filepath)[1]
                            new_filename = f"{clean_title} ({book.id}){file_ext}"
                            
                            old_file_in_renamed = os.path.join(new_book_dir_abs, os.path.basename(fmt.filepath))
                            new_file_in_renamed = os.path.join(new_book_dir_abs, new_filename)
                            if os.path.exists(old_file_in_renamed):
                                os.rename(old_file_in_renamed, new_file_in_renamed)
                                
                            fmt.filename = new_filename
                            fmt.filepath = os.path.join(new_book_dir_rel, new_filename)
                            
                        if book.coverPath:
                            cover_filename = os.path.basename(book.coverPath)
                            book.coverPath = os.path.join(new_book_dir_rel, cover_filename)
                    except Exception as e:
                        print(f"Failed to rename directory: {e}")
                        
        self.db.commit()
        self.db.refresh(book)
        return book

    async def update_cover(self, book_id: str, upload_file) -> Book:
        book = self.get_book_by_id(book_id)
        if not book:
            raise Exception("Book not found")
        if not book.formats:
            raise Exception("Book has no formats files")
            
        first_fmt = book.formats[0]
        book_file_abs = os.path.join(self.library_path, first_fmt.filepath)
        book_dir_abs = os.path.dirname(book_file_abs)
        book_dir_rel = os.path.dirname(first_fmt.filepath)
        
        if book.coverPath:
            old_cover_abs = os.path.join(self.library_path, book.coverPath)
            if os.path.exists(old_cover_abs):
                try:
                    os.remove(old_cover_abs)
                except:
                    pass
                    
        ext = os.path.splitext(upload_file.filename)[1] or ".jpg"
        cover_filename = f"cover{ext}"
        cover_path_abs = os.path.join(book_dir_abs, cover_filename)
        
        with open(cover_path_abs, "wb") as buffer:
            shutil.copyfileobj(upload_file.file, buffer)
            
        book.coverPath = os.path.join(book_dir_rel, cover_filename)
        self.db.commit()
        self.db.refresh(book)
        return book

    def delete_book(self, book_id: str):
        book = self.get_book_by_id(book_id)
        if not book:
            raise Exception("Book not found")
            
        if book.formats:
            first_fmt = book.formats[0]
            book_dir_abs = os.path.dirname(os.path.join(self.library_path, first_fmt.filepath))
            if os.path.exists(book_dir_abs):
                try:
                    shutil.rmtree(book_dir_abs)
                    author_dir_abs = os.path.dirname(book_dir_abs)
                    if os.path.exists(author_dir_abs) and not os.listdir(author_dir_abs):
                        os.rmdir(author_dir_abs)
                except Exception as e:
                    print(f"Failed to delete book files: {e}")
                    
        self.db.delete(book)
        self.db.commit()

    def run_plugin(self, book_id: str, plugin_id: str) -> Book:
        book = self.get_book_by_id(book_id)
        if not book:
            raise Exception("Book not found")
            
        plugin = self.plugins_service.get_plugin_by_id(plugin_id)
        if not plugin:
            raise Exception(f"Plugin {plugin_id} not found")
        if plugin.type != "transformation":
            raise Exception("Plugin is not a transformation plugin")
        if not book.formats:
            raise Exception("No formats to transform")
            
        first_fmt = book.formats[0]
        input_path = os.path.join(self.library_path, first_fmt.filepath)
        
        output_path_abs, output_filename = self.plugins_service.run_transformation_plugin(input_path, plugin_id=plugin_id, db=self.db)
        filesize = os.path.getsize(output_path_abs)
        
        new_book_id = str(uuid.uuid4())
        clean_author = self.sanitize_filename(book.author)
        clean_title = self.sanitize_filename(f"{book.title} ({plugin.name})")
        
        new_book_dir_rel = os.path.join(clean_author, f"{clean_title} ({new_book_id})")
        new_book_dir_abs = os.path.join(self.library_path, new_book_dir_rel)
        os.makedirs(new_book_dir_abs, exist_ok=True)
        
        ext = os.path.splitext(output_path_abs)[1].replace(".", "")
        filename = f"{clean_title} ({new_book_id}).{ext}"
        dest_path_abs = os.path.join(new_book_dir_abs, filename)
        dest_path_rel = os.path.join(new_book_dir_rel, filename)
        
        shutil.move(output_path_abs, dest_path_abs)
        
        cover_path_rel = None
        if book.coverPath:
            old_cover_abs = os.path.join(self.library_path, book.coverPath)
            if os.path.exists(old_cover_abs):
                cover_ext = os.path.splitext(book.coverPath)[1]
                cover_filename = f"cover{cover_ext}"
                new_cover_abs = os.path.join(new_book_dir_abs, cover_filename)
                shutil.copyfile(old_cover_abs, new_cover_abs)
                cover_path_rel = os.path.join(new_book_dir_rel, cover_filename)
                
        new_book = Book(
            id=new_book_id,
            title=f"{book.title} (FR)",
            author=book.author,
            publisher=book.publisher,
            publishDate=book.publishDate,
            description=book.description,
            coverPath=cover_path_rel
        )
        
        tags = [t.name for t in book.tags]
        if "Traduit" not in tags:
            tags.append("Traduit")
            
        for tag_name in tags:
            tag = self.db.query(Tag).filter(Tag.name == tag_name).first()
            if not tag:
                tag = Tag(name=tag_name)
                self.db.add(tag)
            new_book.tags.append(tag)
            
        new_fmt = BookFormat(
            book_id=new_book_id,
            format=ext,
            filename=filename,
            filepath=dest_path_rel,
            filesize=filesize
        )
        new_book.formats.append(new_fmt)
        
        self.db.add(new_book)
        self.db.commit()
        self.db.refresh(new_book)
        return new_book
