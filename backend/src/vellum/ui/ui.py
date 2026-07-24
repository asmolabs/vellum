import reflex as rx
from .state import AppState
from .components.sidebar import sidebar
from .components.header import header
from .components.books_view import books_view
from .components.detail_drawer import detail_drawer
from .components.dialogs import edit_dialog, confirm_delete_dialog, settings_dialog, bulk_edit_dialog, stats_dialog, duplicates_dialog
from .components.reader import reader_overlay

def index() -> rx.Component:
    return rx.box(
        sidebar(),
        rx.box(
            header(),
            books_view(),
            class_name="main-content",
        ),
        detail_drawer(),
        edit_dialog(),
        confirm_delete_dialog(),
        bulk_edit_dialog(),
        stats_dialog(),
        duplicates_dialog(),
        settings_dialog(),
        reader_overlay(),
        class_name=rx.cond(AppState.theme == "light", "app-container light-theme", "app-container"),
        on_mount=AppState.on_load,
    )

app = rx.App(
    stylesheets=["/style.css"]
)
app.add_page(index, route="/")
