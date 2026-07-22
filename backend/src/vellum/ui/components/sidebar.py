import reflex as rx
from ..state import AppState

def sidebar() -> rx.Component:
    return rx.box(
        # Logo section
        rx.box(
            rx.hstack(
                rx.text("📚", class_name="logo-icon"),
                rx.heading("Vellum", class_name="logo-text"),
                align="center",
                spacing="2",
            ),
            rx.button(
                rx.cond(AppState.theme == "light", "🌙", "☀️"),
                on_click=AppState.toggle_theme,
                class_name="theme-toggle-btn",
                title="Changer le thème",
            ),
            class_name="logo-section",
        ),
        # Import button container
        rx.box(
            rx.upload(
                rx.hstack(
                    rx.text("➕ Importer un Livre", font_weight="500", font_size="14px", color="white"),
                    align="center",
                    justify="center",
                ),
                id="import_upload",
                accept={
                    "application/epub+zip": [".epub"],
                    "application/x-mobipocket-ebook": [".mobi"],
                    "application/vnd.amazon.ebook": [".azw3"],
                },
                max_files=1,
                on_drop=AppState.handle_import(rx.upload_files(upload_id="import_upload")),
                class_name="import-label",
                border="none",
                padding="0",
            ),
            class_name="import-btn-container",
        ),

        # Nav Section (Categories)
        rx.box(
            rx.box("Catégories", class_name="nav-title"),
            rx.box(
                rx.box(
                    rx.text("Tous les livres"),
                    rx.text(AppState.total_books_count, class_name="nav-item-count"),
                    class_name=rx.cond(AppState.selected_tag == "", "nav-item active", "nav-item"),
                    on_click=AppState.filter_by_tag(""),
                ),
                rx.foreach(
                    AppState.all_tags,
                    lambda tag: rx.box(
                        rx.text(f"# {tag}"),
                        rx.text(AppState.tag_counts[tag], class_name="nav-item-count"),
                        class_name=rx.cond(AppState.selected_tag == tag, "nav-item active", "nav-item"),
                        on_click=AppState.filter_by_tag(tag),
                    ),
                ),
                class_name="nav-list",
            ),
            class_name="nav-section",
        ),
        class_name=rx.cond(AppState.sidebar_collapsed, "sidebar collapsed", "sidebar"),
    )
