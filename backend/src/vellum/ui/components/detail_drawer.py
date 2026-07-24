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
                                    rx.text(p.name, style={"color": "var(--accent)", "font-weight": "600"}),
                                    rx.text(p.description, style={"font-size": "11px", "color": "var(--text-muted)"}),
                                    class_name="btn-secondary",
                                    on_click=lambda: AppState.run_plugin(p.id),
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
                # Conversion Section
                rx.box(
                    rx.box("🔄 Convertir le Format", class_name="detail-section-title"),
                    rx.hstack(
                        rx.button("EPUB", size="1", variant="soft", on_click=lambda: AppState.convert_book_to_format("epub")),
                        rx.button("MOBI", size="1", variant="soft", on_click=lambda: AppState.convert_book_to_format("mobi")),
                        rx.button("AZW3", size="1", variant="soft", on_click=lambda: AppState.convert_book_to_format("azw3")),
                        rx.button("PDF", size="1", variant="soft", on_click=lambda: AppState.convert_book_to_format("pdf")),
                        rx.button("TXT", size="1", variant="soft", on_click=lambda: AppState.convert_book_to_format("txt")),
                        spacing="2",
                        wrap="wrap",
                    ),
                    rx.cond(
                        AppState.convert_status != "",
                        rx.text(AppState.convert_status, font_size="11px", margin_top="4px", color="var(--accent)"),
                    ),
                    margin_bottom="16px",
                ),

                # USB Device Transfer Section
                rx.box(
                    rx.box("🔌 Transférer sur Liseuse USB", class_name="detail-section-title"),
                    rx.cond(
                        AppState.connected_devices.length() > 0,
                        rx.vstack(
                            rx.button(
                                rx.hstack(
                                    rx.text("📱 Copier sur "),
                                    rx.text(AppState.connected_devices[0]["name"]),
                                    spacing="1",
                                    align="center",
                                ),
                                on_click=AppState.send_to_connected_device,
                                class_name="btn-secondary",
                                size="1",
                                width="100%",
                            ),
                            rx.cond(
                                AppState.device_status != "",
                                rx.text(AppState.device_status, font_size="11px", color="var(--success)"),
                            ),
                            spacing="1",
                            width="100%",
                        ),
                        rx.hstack(
                            rx.text("Aucune liseuse USB détectée", font_size="12px", color="var(--text-muted)"),
                            rx.button("🔄", on_click=AppState.load_connected_devices, size="1", variant="ghost", title="Rafraîchir"),
                            align="center",
                            spacing="2",
                        ),
                    ),
                    margin_bottom="16px",
                ),

                # Send-to-Kindle Section
                rx.box(
                    rx.box("📲 Send-to-Kindle (Email)", class_name="detail-section-title"),
                    rx.vstack(
                        rx.input(
                            placeholder="votre_kindle@kindle.com",
                            value=AppState.kindle_email,
                            on_change=AppState.set_kindle_email,
                            size="1",
                            width="100%",
                        ),
                        rx.button(
                            "📧 Envoyer à ma Kindle",
                            on_click=AppState.send_to_kindle,
                            class_name="btn-secondary",
                            size="1",
                            width="100%",
                        ),
                        rx.cond(
                            AppState.kindle_status != "",
                            rx.text(AppState.kindle_status, font_size="11px", color="var(--accent)"),
                        ),
                        spacing="2",
                        width="100%",
                    ),
                    margin_bottom="16px",
                ),
                # Info Section
                rx.box("Informations", class_name="detail-section-title"),
                rx.cond(
                    book.series != "",
                    rx.box(
                        rx.text.strong("Série: "),
                        book.series,
                        f" (Tome {book.series_index})",
                        class_name="detail-text",
                    ),
                ),
                rx.cond(
                    book.rating > 0,
                    rx.box(
                        rx.text.strong("Note: "),
                        rx.cond(
                            book.rating == 5, "⭐⭐⭐⭐⭐ (5/5)",
                            rx.cond(
                                book.rating == 4, "⭐⭐⭐⭐ (4/5)",
                                rx.cond(
                                    book.rating == 3, "⭐⭐⭐ (3/5)",
                                    rx.cond(
                                        book.rating == 2, "⭐⭐ (2/5)",
                                        "⭐ (1/5)"
                                    )
                                )
                            )
                        ),
                        class_name="detail-text",
                    ),
                ),
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
                rx.cond(
                    book.isbn != "",
                    rx.box(
                        rx.text.strong("ISBN: "),
                        book.isbn,
                        class_name="detail-text",
                    ),
                ),
                rx.cond(
                    book.language != "",
                    rx.box(
                        rx.text.strong("Langue: "),
                        book.language.upper(),
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
