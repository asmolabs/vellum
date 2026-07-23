import os
import json
from typing import Optional, List
from fastapi import FastAPI, HTTPException, UploadFile, File, Depends
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse, Response
from pydantic import BaseModel

from .database.connection import get_session, init_db
from .services.plugins_service import PluginsService
from .services.books_service import BooksService
from .services.device_service import DeviceService
from .services.email_service import EmailService
from .services.opds_service import OPDSService
from .services.converter_service import ConverterService

class KindlePayload(BaseModel):
    kindle_email: str
    smtp_host: str = "smtp.gmail.com"
    smtp_port: int = 587
    smtp_user: str = ""
    smtp_pass: str = ""

class DeviceTransferPayload(BaseModel):
    target_folder: str

app = FastAPI(title="Ebook Library Backend")

# Enable CORS for development
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

CONFIG_FILE = os.path.expanduser("~/.ebook_library_config.json")

# Inversion of Control Container Setup
plugins_service_instance = PluginsService()

class LibraryPathPayload(BaseModel):
    path: str

class BookUpdatePayload(BaseModel):
    title: str
    author: str
    publisher: Optional[str] = None
    publishDate: Optional[str] = None
    description: Optional[str] = None
    tags: List[str] = []
    series: Optional[str] = None
    series_index: Optional[float] = 0.0
    rating: Optional[int] = 0
    isbn: Optional[str] = None
    language: Optional[str] = "fr"

def load_config() -> str:
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                config = json.load(f)
                return config.get("library_path", os.path.expanduser("~/EbookLibrary"))
        except:
            pass
    return os.path.expanduser("~/EbookLibrary")

def save_config(library_path: str):
    try:
        with open(CONFIG_FILE, "w", encoding="utf-8") as f:
            json.dump({"library_path": library_path}, f, ensure_ascii=False, indent=2)
    except Exception as e:
        print(f"Failed to save config: {e}")

# Initialize Library Path
current_library_path = load_config()
os.makedirs(current_library_path, exist_ok=True)
init_db(current_library_path)

# Dependency Injection Providers
def get_db():
    db = get_session()
    try:
        yield db
    finally:
        db.close()

def get_plugins_service() -> PluginsService:
    return plugins_service_instance

def get_books_service(db=Depends(get_db), plugins=Depends(get_plugins_service)) -> BooksService:
    return BooksService(db, plugins, current_library_path)


@app.get("/books/tags")
def get_tags_with_counts(books_service: BooksService = Depends(get_books_service)):
    return books_service.get_tags_with_counts()

@app.get("/books")
def get_books(
    search: Optional[str] = None,
    tag: Optional[str] = None,
    skip: int = 0,
    limit: int = 20,
    books_service: BooksService = Depends(get_books_service)
):
    books = books_service.get_books(search, tag, skip, limit)
    return [b.to_dict() for b in books]

@app.get("/books/library-path")
def get_library_path():
    return {"path": current_library_path}

@app.post("/books/library-path")
def set_library_path(payload: LibraryPathPayload):
    global current_library_path
    new_path = os.path.abspath(payload.path)
    os.makedirs(new_path, exist_ok=True)
    current_library_path = new_path
    save_config(new_path)
    init_db(new_path)
    return {"path": current_library_path}

@app.get("/books/plugins/list")
def get_plugins_list(plugins_service: PluginsService = Depends(get_plugins_service)):
    return plugins_service.get_plugins()

@app.get("/books/plugins/{plugin_id}/config")
def get_plugin_config(
    plugin_id: str,
    db=Depends(get_db),
    plugins_service: PluginsService = Depends(get_plugins_service)
):
    plugin = plugins_service.get_plugin_by_id(plugin_id)
    if not plugin:
        raise HTTPException(status_code=404, detail="Plugin not found")
    values = plugins_service.get_plugin_params(db, plugin_id)
    return {
        "plugin_id": plugin_id,
        "name": plugin.name,
        "description": plugin.description,
        "schema": plugin.config_schema,
        "values": values
    }

@app.post("/books/plugins/{plugin_id}/config")
def save_plugin_config(
    plugin_id: str,
    payload: dict,
    db=Depends(get_db),
    plugins_service: PluginsService = Depends(get_plugins_service)
):
    plugin = plugins_service.get_plugin_by_id(plugin_id)
    if not plugin:
        raise HTTPException(status_code=404, detail="Plugin not found")
    saved = plugins_service.save_plugin_params(db, plugin_id, payload)
    return {"success": True, "values": saved}

@app.get("/books/{book_id}")
def get_book(book_id: str, books_service: BooksService = Depends(get_books_service)):
    book = books_service.get_book_by_id(book_id)
    if not book:
        raise HTTPException(status_code=404, detail="Book not found")
    return book.to_dict()

@app.get("/books/{book_id}/file/{format}")
def get_book_file(book_id: str, format: str, books_service: BooksService = Depends(get_books_service)):
    fmt = books_service.get_book_file(book_id, format)
    if not fmt:
        raise HTTPException(status_code=404, detail="Format not found")
    
    abs_path = os.path.join(current_library_path, fmt.filepath)
    if not os.path.exists(abs_path):
        raise HTTPException(status_code=404, detail="Book file not found on disk")
        
    return FileResponse(abs_path, media_type="application/octet-stream", filename=fmt.filename)

@app.get("/books/{book_id}/cover")
def get_book_cover(book_id: str, books_service: BooksService = Depends(get_books_service)):
    book = books_service.get_book_by_id(book_id)
    if not book or not book.coverPath:
        raise HTTPException(status_code=404, detail="Cover not found")
        
    abs_path = os.path.join(current_library_path, book.coverPath)
    if not os.path.exists(abs_path):
        raise HTTPException(status_code=404, detail="Cover file not found on disk")
        
    return FileResponse(abs_path)

@app.post("/books/import")
async def import_book(file: UploadFile = File(...), books_service: BooksService = Depends(get_books_service)):
    try:
        book = await books_service.import_book(file)
        return book.to_dict()
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

@app.put("/books/{book_id}")
def update_book(book_id: str, payload: BookUpdatePayload, books_service: BooksService = Depends(get_books_service)):
    try:
        book = books_service.update_book(
            book_id,
            payload.title,
            payload.author,
            payload.publisher,
            payload.publishDate,
            payload.description,
            payload.tags,
            payload.series,
            payload.series_index,
            payload.rating,
            payload.isbn,
            payload.language
        )
        return book.to_dict()
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

@app.post("/books/{book_id}/cover")
async def update_cover(book_id: str, cover: UploadFile = File(...), books_service: BooksService = Depends(get_books_service)):
    try:
        book = await books_service.update_cover(book_id, cover)
        return book.to_dict()
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

@app.delete("/books/{book_id}")
def delete_book(book_id: str, books_service: BooksService = Depends(get_books_service)):
    try:
        books_service.delete_book(book_id)
        return {"success": True}
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

@app.post("/books/{book_id}/plugin/{plugin_id}")
def run_plugin(book_id: str, plugin_id: str, books_service: BooksService = Depends(get_books_service)):
    try:
        book = books_service.run_plugin(book_id, plugin_id)
        return book.to_dict()
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# --- Phase 2: USB Device Transfer & Send-to-Kindle ---
@app.get("/devices")
def list_devices():
    return DeviceService.list_connected_devices()

@app.post("/books/{book_id}/send-to-device")
def send_to_device(book_id: str, payload: DeviceTransferPayload, books_service: BooksService = Depends(get_books_service)):
    book = books_service.get_book_by_id(book_id)
    if not book or not book.formats:
        raise HTTPException(status_code=404, detail="Book or file format not found")
    first_fmt = book.formats[0]
    abs_filepath = os.path.join(current_library_path, first_fmt.filepath)
    try:
        dest = DeviceService.send_to_device(abs_filepath, payload.target_folder)
        return {"success": True, "destination": dest}
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

@app.post("/books/{book_id}/send-to-kindle")
def send_to_kindle(book_id: str, payload: KindlePayload, books_service: BooksService = Depends(get_books_service)):
    book = books_service.get_book_by_id(book_id)
    if not book or not book.formats:
        raise HTTPException(status_code=404, detail="Book or file format not found")
    first_fmt = book.formats[0]
    abs_filepath = os.path.join(current_library_path, first_fmt.filepath)
    try:
        EmailService.send_to_kindle(
            smtp_host=payload.smtp_host,
            smtp_port=payload.smtp_port,
            smtp_user=payload.smtp_user,
            smtp_pass=payload.smtp_pass,
            kindle_email=payload.kindle_email,
            book_filepath=abs_filepath
        )
        return {"success": True, "message": f"Book sent to {payload.kindle_email}"}
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# --- Phase 3: OPDS 1.2 Feed ---
@app.get("/opds/v1/catalog.xml")
def get_opds_catalog(books_service: BooksService = Depends(get_books_service)):
    all_books = books_service.get_books(limit=500)
    base_url = "http://127.0.0.1:8001"
    xml_content = OPDSService.generate_opds_feed(all_books, base_url)
    return Response(content=xml_content, media_type="application/atom+xml")

# --- Phase 4: Universal Converter ---
@app.post("/books/{book_id}/convert/{target_format}")
def convert_book_format(book_id: str, target_format: str, books_service: BooksService = Depends(get_books_service)):
    book = books_service.get_book_by_id(book_id)
    if not book or not book.formats:
        raise HTTPException(status_code=404, detail="Book or format not found")
    first_fmt = book.formats[0]
    abs_filepath = os.path.join(current_library_path, first_fmt.filepath)
    try:
        out_abs, out_filename = ConverterService.convert_ebook(abs_filepath, target_format)
        filesize = os.path.getsize(out_abs)
        rel_filepath = os.path.relpath(out_abs, current_library_path)

        # Register format in DB
        from .models.book import BookFormat
        new_fmt = BookFormat(
            book_id=book.id,
            format=target_format.lower(),
            filename=out_filename,
            filepath=rel_filepath,
            filesize=filesize
        )
        books_service.db.add(new_fmt)
        books_service.db.commit()
        books_service.db.refresh(book)
        return book.to_dict()
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# --- Phase 5: Reading Progress ---
@app.post("/books/{book_id}/progress")
def update_progress(book_id: str, progress: float, books_service: BooksService = Depends(get_books_service)):
    try:
        book = books_service.update_reading_progress(book_id, progress)
        return book.to_dict()
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


