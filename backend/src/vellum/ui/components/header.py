import reflex as rx
from ..state import AppState

def header() -> rx.Component:
    return rx.box(
        # Hamburger & Search
        rx.hstack(
            rx.button(
                "☰",
                on_click=AppState.toggle_sidebar,
                class_name="hamburger-btn",
                title="Masquer/Afficher la barre latérale",
            ),
            rx.box(
                rx.text("🔍", class_name="search-icon"),
                rx.input(
                    placeholder="Rechercher par titre, auteur ou description...",
                    class_name="search-input",
                    value=AppState.search_query,
                    on_change=AppState.set_search_query,
                ),
                class_name="search-bar",
            ),
            align="center",
            spacing="4",
            flex="1",
        ),
        # Action Buttons
        rx.hstack(
            rx.upload(
                rx.button("📥", class_name="header-action-btn", title="Importer un livre"),
                id="header_import_upload",
                accept={
                    "application/epub+zip": [".epub"],
                    "application/x-mobipocket-ebook": [".mobi"],
                    "application/vnd.amazon.ebook": [".azw3"],
                },
                max_files=1,
                on_drop=AppState.handle_import(rx.upload_files(upload_id="header_import_upload")),
                border="none",
                padding="0",
            ),
            rx.button(
                "⚙️",
                on_click=AppState.open_settings_dialog,
                class_name="header-action-btn",
                title="Configuration & Paramètres",
            ),
            spacing="2",
            margin_right="4",
        ),
        # View Mode Toggle
        rx.box(
            rx.button(
                "📱 Grille",
                on_click=AppState.set_view_mode("grid"),
                class_name=rx.cond(AppState.view_mode == "grid", "view-btn active", "view-btn"),
            ),
            rx.button(
                "☰ Liste",
                on_click=AppState.set_view_mode("list"),
                class_name=rx.cond(AppState.view_mode == "list", "view-btn active", "view-btn"),
                margin_left="6px",
            ),
            class_name="view-controls",
        ),
        class_name="header",
    )
