import os
import json
import httpx
import reflex as rx
from pydantic import BaseModel

DEFAULT_API_URL = os.environ.get("VELLUM_API_URL", "http://127.0.0.1:8001")

class BookFormatModel(BaseModel):
    format: str = ""
    filename: str = ""
    filepath: str = ""
    filesize: int = 0

class BookModel(BaseModel):
    id: str = ""
    title: str = ""
    author: str = ""
    publisher: str = ""
    publishDate: str = ""
    description: str = ""
    coverPath: str = ""
    series: str = ""
    series_index: float = 0.0
    rating: int = 0
    isbn: str = ""
    language: str = "fr"
    tags: list[str] = []
    formats: list[BookFormatModel] = []

class PluginFieldModel(BaseModel):
    key: str = ""
    label: str = ""
    type: str = "text"
    options: list[str] = []
    default: str = ""
    description: str = ""

class PluginModel(BaseModel):
    id: str = ""
    name: str = ""
    description: str = ""
    version: str = ""
    type: str = ""
    category: str = ""
    config_schema: list[PluginFieldModel] = []

class TopAuthorModel(BaseModel):
    author: str = ""
    count: int = 0

class AnalyticsModel(BaseModel):
    total_books: int = 0
    total_formats: int = 0
    total_size_mb: float = 0.0
    read_count: int = 0
    reading_count: int = 0
    unread_count: int = 0
    top_authors: list[TopAuthorModel] = []

class DuplicateItemModel(BaseModel):
    original: BookModel = BookModel()
    duplicate: BookModel = BookModel()
    reason: str = ""

class AppState(rx.State):
    books: list[BookModel] = []
    library_path: str = ""
    search_query: str = ""
    selected_book: BookModel = BookModel()
    has_selected_book: bool = False
    selected_tag: str = ""
    all_tags: list[str] = []
    tag_counts: dict[str, int] = {}
    total_books_count: int = 0
    view_mode: str = "grid"  # "grid" or "list"
    sidebar_collapsed: bool = False
    theme: str = "dark"      # "dark" or "light"
    plugins: list[PluginModel] = []
    is_plugin_running: bool = False

    # Multi-selection & Bulk operations
    selected_book_ids: list[str] = []
    show_bulk_edit_dialog: bool = False
    bulk_author: str = ""
    bulk_series: str = ""
    bulk_rating: int = -1
    bulk_tags: str = ""
    bulk_status: str = ""

    # Smart Filters
    filter_rating_min: int = 0
    filter_status: str = "all"  # "all", "unread", "reading", "read"

    # Analytics & Duplicates Dialog States
    show_stats_dialog: bool = False
    show_duplicates_dialog: bool = False
    analytics_data: AnalyticsModel = AnalyticsModel()
    duplicates_data: list[DuplicateItemModel] = []

    # Dialog states
    show_edit_dialog: bool = False
    show_confirm_delete: bool = False
    show_settings_dialog: bool = False

    def open_settings_dialog(self):
        self.show_settings_dialog = True

    def close_settings_dialog(self):
        self.show_settings_dialog = False

    # Edit form state
    edit_title: str = ""
    edit_author: str = ""
    edit_publisher: str = ""
    edit_publish_date: str = ""
    edit_description: str = ""
    edit_tags: str = ""
    edit_series: str = ""
    edit_series_index: str = "1.0"
    edit_rating: int = 0
    edit_isbn: str = ""
    edit_language: str = "fr"

    # EPUB Reader state
    reading_book: dict = {}
    is_reading: bool = False
    font_size: int = 100

    api_url: str = DEFAULT_API_URL

    plugin_configs: dict[str, dict] = {}
    plugin_schemas: dict[str, list[dict]] = {}
    active_plugin_id: str = ""
    plugin_save_status: str = ""

    # USB Devices & Kindle
    connected_devices: list[dict] = []
    selected_device_folder: str = ""
    kindle_email: str = ""
    smtp_host: str = "smtp.gmail.com"
    smtp_port: int = 587
    smtp_user: str = ""
    smtp_pass: str = ""
    kindle_status: str = ""
    device_status: str = ""

    # Conversion & OPDS
    convert_status: str = ""
    selected_convert_format: str = "mobi"
    opds_url: str = "http://127.0.0.1:8001/opds/v1/catalog.xml"

    # Setters
    def set_search_query(self, val: str):
        self.search_query = val

    def set_edit_title(self, val: str):
        self.edit_title = val

    def set_edit_author(self, val: str):
        self.edit_author = val

    def set_edit_publisher(self, val: str):
        self.edit_publisher = val

    def set_edit_publish_date(self, val: str):
        self.edit_publish_date = val

    def set_edit_description(self, val: str):
        self.edit_description = val

    def set_edit_tags(self, val: str):
        self.edit_tags = val

    def set_edit_series(self, val: str):
        self.edit_series = val

    def set_edit_series_index(self, val: str):
        self.edit_series_index = val

    def set_edit_rating(self, val: int):
        self.edit_rating = val

    def set_edit_isbn(self, val: str):
        self.edit_isbn = val

    def set_edit_language(self, val: str):
        self.edit_language = val

    def set_kindle_email(self, val: str):
        self.kindle_email = val

    def set_smtp_host(self, val: str):
        self.smtp_host = val

    def set_smtp_port(self, val: str):
        if val.isdigit():
            self.smtp_port = int(val)

    def set_smtp_user(self, val: str):
        self.smtp_user = val

    def set_smtp_pass(self, val: str):
        self.smtp_pass = val

    def set_bulk_author(self, val: str):
        self.bulk_author = val

    def set_bulk_series(self, val: str):
        self.bulk_series = val

    def set_bulk_tags(self, val: str):
        self.bulk_tags = val

    async def on_load(self):
        """Initial load handler."""
        await self.load_library_path()
        await self.load_tags()
        await self.load_books()
        await self.load_plugins()
        await self.load_plugin_configs()
        await self.load_connected_devices()

    async def load_connected_devices(self):
        try:
            async with httpx.AsyncClient() as client:
                res = await client.get(f"{self.api_url}/devices")
                if res.status_code == 200:
                    self.connected_devices = res.json()
                    if self.connected_devices:
                        self.selected_device_folder = self.connected_devices[0].get("target_folder", "")
        except Exception as e:
            print(f"Error loading devices: {e}")

    async def send_to_connected_device(self):
        if not self.has_selected_book or not self.selected_device_folder:
            self.device_status = "❌ Veuillez sélectionner un livre et un appareil."
            return
        book_id = self.selected_book.id
        try:
            async with httpx.AsyncClient() as client:
                res = await client.post(
                    f"{self.api_url}/books/{book_id}/send-to-device",
                    json={"target_folder": self.selected_device_folder}
                )
                if res.status_code == 200:
                    self.device_status = "✅ Livre copié avec succès sur la liseuse !"
                else:
                    self.device_status = f"❌ Erreur: {res.json().get('detail')}"
        except Exception as e:
            self.device_status = f"❌ Échec de la copie: {e}"

    async def send_to_kindle(self):
        if not self.has_selected_book or not self.kindle_email:
            self.kindle_status = "❌ Veuillez saisir une adresse e-mail Kindle."
            return
        book_id = self.selected_book.id
        payload = {
            "kindle_email": self.kindle_email,
            "smtp_host": self.smtp_host,
            "smtp_port": int(self.smtp_port),
            "smtp_user": self.smtp_user,
            "smtp_pass": self.smtp_pass
        }
        self.kindle_status = "⏳ Envoi du livre par e-mail en cours..."
        try:
            async with httpx.AsyncClient(timeout=30.0) as client:
                res = await client.post(f"{self.api_url}/books/{book_id}/send-to-kindle", json=payload)
                if res.status_code == 200:
                    self.kindle_status = f"✅ Livre envoyé à {self.kindle_email} !"
                else:
                    self.kindle_status = f"❌ Erreur d'envoi: {res.json().get('detail')}"
        except Exception as e:
            self.kindle_status = f"❌ Échec de l'envoi Kindle: {e}"

    async def convert_book_to_format(self, target_fmt: str):
        if not self.has_selected_book:
            return
        book_id = self.selected_book.id
        self.convert_status = f"⏳ Conversion du livre vers {target_fmt.upper()} en cours..."
        try:
            async with httpx.AsyncClient(timeout=60.0) as client:
                res = await client.post(f"{self.api_url}/books/{book_id}/convert/{target_fmt}")
                if res.status_code == 200:
                    raw = res.json()
                    fmts = [BookFormatModel(**f) for f in raw.get("formats", [])]
                    raw["formats"] = fmts
                    self.selected_book = BookModel(**raw)
                else:
                    self.convert_status = f"❌ Erreur: {res.json().get('detail')}"
        except Exception as e:
            self.convert_status = f"❌ Échec de la conversion: {e}"

    def toggle_select_book_id(self, book_id: str):
        ids = list(self.selected_book_ids)
        if book_id in ids:
            ids.remove(book_id)
        else:
            ids.append(book_id)
        self.selected_book_ids = ids

    def select_all_visible_books(self):
        self.selected_book_ids = [b.id for b in self.books]

    def clear_book_selection(self):
        self.selected_book_ids = []

    def open_bulk_edit_dialog(self):
        if not self.selected_book_ids:
            return
        self.bulk_author = ""
        self.bulk_series = ""
        self.bulk_rating = -1
        self.bulk_tags = ""
        self.bulk_status = ""
        self.show_bulk_edit_dialog = True

    def close_bulk_edit_dialog(self):
        self.show_bulk_edit_dialog = False

    def set_bulk_rating(self, rating: int):
        self.bulk_rating = rating

    async def save_bulk_edit(self):
        if not self.selected_book_ids:
            return
        tags_list = [t.strip() for t in self.bulk_tags.split(",") if t.strip()]
        payload = {
            "book_ids": self.selected_book_ids,
            "author": self.bulk_author or None,
            "series": self.bulk_series or None,
            "rating": self.bulk_rating if self.bulk_rating >= 0 else None,
            "add_tags": tags_list
        }
        try:
            async with httpx.AsyncClient() as client:
                res = await client.post(f"{self.api_url}/books/bulk-update", json=payload)
                if res.status_code == 200:
                    self.show_bulk_edit_dialog = False
                    self.clear_book_selection()
                    await self.load_tags()
                    await self.load_books()
        except Exception as e:
            self.bulk_status = f"❌ Échec: {e}"

    async def delete_selected_books(self):
        if not self.selected_book_ids:
            return
        try:
            async with httpx.AsyncClient() as client:
                res = await client.post(f"{self.api_url}/books/bulk-delete", json={"book_ids": self.selected_book_ids})
                if res.status_code == 200:
                    self.clear_book_selection()
                    await self.load_tags()
                    await self.load_books()
        except Exception as e:
            print(f"Error bulk deleting: {e}")

    async def open_stats_dialog(self):
        try:
            async with httpx.AsyncClient() as client:
                res = await client.get(f"{self.api_url}/analytics/stats")
                if res.status_code == 200:
                    raw = res.json()
                    authors = [TopAuthorModel(**a) for a in raw.get("top_authors", [])]
                    raw["top_authors"] = authors
                    self.analytics_data = AnalyticsModel(**raw)
                    self.show_stats_dialog = True
        except Exception as e:
            print(f"Error loading analytics: {e}")

    def close_stats_dialog(self):
        self.show_stats_dialog = False

    async def open_duplicates_dialog(self):
        try:
            async with httpx.AsyncClient() as client:
                res = await client.get(f"{self.api_url}/books/duplicates")
                if res.status_code == 200:
                    raw_list = res.json()
                    parsed = []
                    for item in raw_list:
                        orig = BookModel(**item.get("original", {}))
                        dupe = BookModel(**item.get("duplicate", {}))
                        parsed.append(DuplicateItemModel(original=orig, duplicate=dupe, reason=item.get("reason", "")))
                    self.duplicates_data = parsed
                    self.show_duplicates_dialog = True
        except Exception as e:
            print(f"Error loading duplicates: {e}")

    def close_duplicates_dialog(self):
        self.show_duplicates_dialog = False

    async def load_plugin_configs(self):
        try:
            async with httpx.AsyncClient() as client:
                configs = dict(self.plugin_configs)
                for p in self.plugins:
                    p_id = p.id
                    if not p_id:
                        continue
                    res = await client.get(f"{self.api_url}/books/plugins/{p_id}/config")
                    if res.status_code == 200:
                        data = res.json()
                        configs[p_id] = data.get("values", {})
                self.plugin_configs = configs
                if self.plugins and not self.active_plugin_id:
                    self.active_plugin_id = self.plugins[0].id
        except Exception as e:
            print(f"Error loading plugin configs: {e}")

    def select_active_plugin(self, plugin_id: str):
        self.active_plugin_id = plugin_id
        self.plugin_save_status = ""

    def update_plugin_field(self, plugin_id: str, field_key: str, val: str):
        configs = dict(self.plugin_configs)
        if plugin_id not in configs:
            configs[plugin_id] = {}
        sub = dict(configs[plugin_id])
        sub[field_key] = val
        configs[plugin_id] = sub
        self.plugin_configs = configs

    async def save_active_plugin_config(self, plugin_id: str):
        values = self.plugin_configs.get(plugin_id, {})
        try:
            async with httpx.AsyncClient() as client:
                res = await client.post(f"{self.api_url}/books/plugins/{plugin_id}/config", json=values)
                if res.status_code == 200:
                    self.plugin_save_status = "✅ Paramètres enregistrés avec succès !"
        except Exception as e:
            self.plugin_save_status = f"❌ Échec de la sauvegarde : {e}"

    async def load_library_path(self):
        try:
            async with httpx.AsyncClient() as client:
                res = await client.get(f"{self.api_url}/books/library-path")
                if res.status_code == 200:
                    self.library_path = res.json().get("path", "")
        except Exception as e:
            print(f"Error loading library path: {e}")

    async def load_tags(self):
        try:
            async with httpx.AsyncClient() as client:
                res = await client.get(f"{self.api_url}/books/tags")
                if res.status_code == 200:
                    data = res.json()
                    self.total_books_count = data.get("total", 0)
                    self.tag_counts = data.get("tags", {})
                    self.all_tags = list(self.tag_counts.keys())
        except Exception as e:
            print(f"Error loading tags: {e}")

    async def load_books(self):
        try:
            url = f"{self.api_url}/books?skip=0&limit=100"
            if self.search_query:
                url += f"&search={self.search_query}"
            if self.selected_tag:
                url += f"&tag={self.selected_tag}"

            async with httpx.AsyncClient() as client:
                res = await client.get(url)
                if res.status_code == 200:
                    raw_books = res.json()
                    parsed = []
                    for b in raw_books:
                        fmts = [BookFormatModel(**f) for f in b.get("formats", [])]
                        b_copy = dict(b)
                        b_copy["formats"] = fmts
                        parsed.append(BookModel(**b_copy))
                    self.books = parsed
        except Exception as e:
            print(f"Error loading books: {e}")

    async def load_plugins(self):
        try:
            async with httpx.AsyncClient() as client:
                res = await client.get(f"{self.api_url}/books/plugins/list")
                if res.status_code == 200:
                    raw = res.json()
                    parsed = []
                    for p in raw:
                        fields = [PluginFieldModel(**f) for f in p.get("config_schema", [])]
                        p_copy = dict(p)
                        p_copy["config_schema"] = fields
                        parsed.append(PluginModel(**p_copy))
                    self.plugins = parsed
        except Exception as e:
            print(f"Error loading plugins: {e}")

    def set_search_query(self, query: str):
        self.search_query = query
        return AppState.load_books

    def select_book(self, book: BookModel):
        self.selected_book = book
        self.has_selected_book = True

    def close_detail(self):
        self.selected_book = BookModel()
        self.has_selected_book = False

    async def filter_by_tag(self, tag: str):
        self.selected_tag = tag
        await self.load_books()

    def toggle_sidebar(self):
        self.sidebar_collapsed = not self.sidebar_collapsed

    def toggle_theme(self):
        self.theme = "light" if self.theme == "dark" else "dark"

    def set_view_mode(self, mode: str):
        self.view_mode = mode

    def set_edit_title(self, val: str):
        self.edit_title = val

    def set_edit_author(self, val: str):
        self.edit_author = val

    def set_edit_publisher(self, val: str):
        self.edit_publisher = val

    def set_edit_publish_date(self, val: str):
        self.edit_publish_date = val

    def set_edit_tags(self, val: str):
        self.edit_tags = val

    def set_edit_description(self, val: str):
        self.edit_description = val

    def open_edit_dialog(self):
        if not self.has_selected_book:
            return
        self.edit_title = self.selected_book.title
        self.edit_author = self.selected_book.author
        self.edit_publisher = self.selected_book.publisher or ""
        self.edit_publish_date = self.selected_book.publishDate or ""
        self.edit_description = self.selected_book.description or ""
        self.edit_tags = ", ".join(self.selected_book.tags) if self.selected_book.tags else ""
        self.edit_series = self.selected_book.series or ""
        self.edit_series_index = str(self.selected_book.series_index or 1.0)
        self.edit_rating = self.selected_book.rating or 0
        self.edit_isbn = self.selected_book.isbn or ""
        self.edit_language = self.selected_book.language or "fr"
        self.show_edit_dialog = True

    def set_edit_rating(self, val: int):
        self.edit_rating = val

    def close_edit_dialog(self):
        self.show_edit_dialog = False

    async def save_metadata(self):
        if not self.has_selected_book:
            return
        book_id = self.selected_book.id
        tags_list = [t.strip() for t in self.edit_tags.split(",") if t.strip()]

        try:
            s_idx = float(self.edit_series_index)
        except ValueError:
            s_idx = 1.0

        payload = {
            "title": self.edit_title,
            "author": self.edit_author,
            "publisher": self.edit_publisher or None,
            "publishDate": self.edit_publish_date or None,
            "description": self.edit_description or None,
            "tags": tags_list,
            "series": self.edit_series or None,
            "series_index": s_idx,
            "rating": self.edit_rating,
            "isbn": self.edit_isbn or None,
            "language": self.edit_language or "fr"
        }

        try:
            async with httpx.AsyncClient() as client:
                res = await client.put(f"{self.api_url}/books/{book_id}", json=payload)
                if res.status_code == 200:
                    raw = res.json()
                    fmts = [BookFormatModel(**f) for f in raw.get("formats", [])]
                    raw["formats"] = fmts
                    self.selected_book = BookModel(**raw)
                    self.show_edit_dialog = False
                    await self.load_tags()
                    await self.load_books()
        except Exception as e:
            print(f"Error saving metadata: {e}")

    def open_confirm_delete(self):
        self.show_confirm_delete = True

    def cancel_delete(self):
        self.show_confirm_delete = False

    async def confirm_delete_book(self):
        if not self.has_selected_book:
            return
        book_id = self.selected_book.id
        try:
            async with httpx.AsyncClient() as client:
                res = await client.delete(f"{self.api_url}/books/{book_id}")
                if res.status_code == 200:
                    self.selected_book = BookModel()
                    self.has_selected_book = False
                    self.show_confirm_delete = False
                    await self.load_tags()
                    await self.load_books()
        except Exception as e:
            print(f"Error deleting book: {e}")

    async def run_plugin(self, plugin_id: str):
        if not self.has_selected_book:
            return
        book_id = self.selected_book.id
        self.is_plugin_running = True
        try:
            async with httpx.AsyncClient(timeout=60.0) as client:
                res = await client.post(f"{self.api_url}/books/{book_id}/plugin/{plugin_id}")
                if res.status_code == 200:
                    raw = res.json()
                    fmts = [BookFormatModel(**f) for f in raw.get("formats", [])]
                    raw["formats"] = fmts
                    self.selected_book = BookModel(**raw)
                    await self.load_tags()
                    await self.load_books()
        except Exception as e:
            print(f"Error running plugin: {e}")
        finally:
            self.is_plugin_running = False

    async def handle_import(self, files: list[rx.UploadFile]):
        if not files:
            return
        file = files[0]
        try:
            file_bytes = await file.read()
            async with httpx.AsyncClient(timeout=60.0) as client:
                files_payload = {"file": (file.filename, file_bytes, "application/octet-stream")}
                res = await client.post(f"{self.api_url}/books/import", files=files_payload)
                if res.status_code == 200:
                    raw = res.json()
                    fmts = [BookFormatModel(**f) for f in raw.get("formats", [])]
                    raw["formats"] = fmts
                    self.selected_book = BookModel(**raw)
                    self.has_selected_book = True
                    await self.load_tags()
                    await self.load_books()
        except Exception as e:
            print(f"Error importing book: {e}")

    async def handle_cover_upload(self, files: list[rx.UploadFile]):
        if not files or not self.has_selected_book:
            return
        file = files[0]
        book_id = self.selected_book.id
        try:
            file_bytes = await file.read()
            async with httpx.AsyncClient(timeout=60.0) as client:
                files_payload = {"cover": (file.filename, file_bytes, "image/jpeg")}
                res = await client.post(f"{self.api_url}/books/{book_id}/cover", files=files_payload)
                if res.status_code == 200:
                    raw = res.json()
                    fmts = [BookFormatModel(**f) for f in raw.get("formats", [])]
                    raw["formats"] = fmts
                    self.selected_book = BookModel(**raw)
                    await self.load_books()
        except Exception as e:
            print(f"Error uploading cover: {e}")

    async def set_library_path_action(self, new_path: str):
        if not new_path:
            return
        try:
            async with httpx.AsyncClient() as client:
                res = await client.post(f"{self.api_url}/books/library-path", json={"path": new_path})
                if res.status_code == 200:
                    self.library_path = res.json().get("path", new_path)
                    self.selected_book = BookModel()
                    self.has_selected_book = False
                    await self.load_tags()
                    await self.load_books()
        except Exception as e:
            print(f"Error changing library path: {e}")

    # EPUB Reader Actions
    def open_reader(self, book: dict):
        formats = book.get("formats", [])
        has_epub = any((isinstance(f, dict) and f.get("format") == "epub") or (isinstance(f, BookFormatModel) and f.format == "epub") for f in formats)
        if not has_epub:
            return rx.window_alert("La liseuse intégrée ne prend en charge que le format EPUB pour l'instant.")
        self.reading_book = book
        self.is_reading = True

    def close_reader(self):
        self.is_reading = False
        self.reading_book = {}

    def increase_font_size(self):
        self.font_size = min(250, self.font_size + 10)

    def decrease_font_size(self):
        self.font_size = max(60, self.font_size - 10)
