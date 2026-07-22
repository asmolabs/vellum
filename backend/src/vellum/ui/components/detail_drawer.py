import reflex as rx
from ..state import AppState

def detail_drawer() -> rx.Component:
    book = AppState.selected_book
    cover_url = f"{AppState.api_url}/books/" + book.id + "/cover"

    return rx.box(
        rx.cond(
            AppState.has_selected_book,
            rx.box(
                # Detail Header
                rx.box(
                    rx.text("Détails du Livre", class_name="nav-title"),
                    rx.button("✕", on_click=AppState.close_detail, class_name="close-btn"),
                    class_name="detail-header",
                ),
                # Cover
                rx.cond(
                    book.coverPath != "",
                    rx.image(src=cover_url, class_name="detail-cover", alt="Couverture"),
                    rx.box(
                        rx.text("📖", style={"font-size": "64px"}),
                        rx.text("Pas de Couverture"),
                        class_name="detail-cover",
                        style={
                            "display": "flex",
                            "flex-direction": "column",
                            "align-items": "center",
                            "justify-content": "center",
                            "background": "#1e1e28",
                            "color": "var(--text-muted)",
                            "gap": "12px",
                        },
                    ),
                ),
                # Title & Author
                rx.heading(book.title, class_name="detail-title"),
                rx.box(f"par {book.author}", class_name="detail-author"),
                # Actions Row
                rx.box(
                    rx.button("Lire", on_click=lambda: AppState.open_reader(book), class_name="btn-primary"),
                    rx.button("Éditer", on_click=AppState.open_edit_dialog, class_name="btn-secondary"),
                    class_name="actions-row",
                ),
                # Plugin indicator
                rx.cond(
                    AppState.is_plugin_running,
                    rx.box(
                        rx.text("⏳ Opération de plugin en cours..."),
                        style={
                            "color": "var(--accent)",
                            "font-size": "13px",
                            "font-weight": "500",
                            "margin-bottom": "12px",
                            "display": "flex",
                            "align-items": "center",
                            "gap": "8px",
                            "background": "rgba(139, 92, 246, 0.1)",
                            "padding": "8px",
                            "border-radius": "6px",
                            "border": "1px solid rgba(139, 92, 246, 0.2)",
                        },
                    ),
                ),
                # Plugins List
                rx.cond(
                    AppState.plugins.length() > 0,
                    rx.box(
                        rx.box("Plugins & Transformations", class_name="detail-section-title"),
                        rx.box(
                            rx.foreach(
                                AppState.plugins,
                                lambda p: rx.button(
                                    rx.text(p["name"], style={"color": "var(--accent)", "font-weight": "600"}),
                                    rx.text(p["description"], style={"font-size": "11px", "color": "var(--text-muted)"}),
                                    class_name="btn-secondary",
                                    on_click=lambda: AppState.run_plugin(p["id"]),
                                    disabled=AppState.is_plugin_running,
                                    style={
                                        "text-align": "left",
                                        "font-size": "13px",
                                        "padding": "10px",
                                        "display": "flex",
                                        "flex-direction": "column",
                                        "align-items": "flex-start",
                                        "gap": "4px",
                                        "width": "100%",
                                    },
                                ),
                            ),
                            style={"display": "flex", "flex-direction": "column", "gap": "8px", "margin-bottom": "20px"},
                        ),
                    ),
                ),
                # Info Section
                rx.box("Informations", class_name="detail-section-title"),
                rx.cond(
                    book.publisher != "",
                    rx.box(
                        rx.text.strong("Éditeur: "),
                        book.publisher,
                        class_name="detail-text",
                    ),
                ),
                rx.cond(
                    book.publishDate != "",
                    rx.box(
                        rx.text.strong("Date de pub.: "),
                        book.publishDate,
                        class_name="detail-text",
                    ),
                ),
                # Tags Section
                rx.cond(
                    book.tags.length() > 0,
                    rx.box(
                        rx.box("Tags", class_name="detail-section-title"),
                        rx.box(
                            rx.foreach(
                                book.tags,
                                lambda t: rx.text(t, class_name="tag-badge"),
                            ),
                            class_name="tag-list",
                        ),
                    ),
                ),
                # Description Section
                rx.cond(
                    book.description != "",
                    rx.box(
                        rx.box("Description", class_name="detail-section-title"),
                        rx.box(book.description, class_name="detail-text", style={"font-size": "13px"}),
                    ),
                ),
                # Delete Button
                rx.button(
                    "Supprimer le livre",
                    on_click=AppState.open_confirm_delete,
                    class_name="btn-secondary",
                    style={"border-color": "var(--danger)", "color": "var(--danger)", "width": "100%", "margin-top": "24px"},
                ),
            ),
        ),
        class_name=rx.cond(AppState.has_selected_book, "detail-drawer", "detail-drawer hidden"),
    )
