import reflex as rx
from ..state import AppState, BookModel

def book_card(book: BookModel) -> rx.Component:
    cover_url = f"{AppState.api_url}/books/" + book.id + "/cover"

    return rx.box(
        # Cover Wrapper
        rx.box(
            rx.cond(
                book.coverPath != "",
                rx.image(src=cover_url, class_name="book-cover", alt="Couverture"),
                rx.box(
                    rx.text("📖", font_size="40px"),
                    rx.text("Pas de Couverture", font_size="11px", font_weight="500"),
                    class_name="no-cover-placeholder",
                ),
            ),
            class_name="cover-wrapper",
        ),
        # Book Card Info
        rx.box(
            rx.box(book.title, class_name="book-card-title", title=book.title),
            rx.box(book.author, class_name="book-card-author", title=book.author),
            rx.box(
                rx.text(
                    rx.cond(
                        book.formats.length() > 0,
                        book.formats[0].format.upper(),
                        "EPUB"
                    ),
                    class_name="format-badge",
                ),
                class_name="book-card-formats",
            ),
            class_name="book-card-info",
        ),
        class_name=rx.cond(AppState.selected_book.id == book.id, "book-card selected", "book-card"),
        on_click=lambda: AppState.select_book(book),
    )

def books_grid() -> rx.Component:
    return rx.cond(
        AppState.books.length() == 0,
        rx.box(
            rx.text("📚", font_size="48px", margin_bottom="12px"),
            rx.text("Aucun livre trouvé dans la bibliothèque."),
            style={
                "display": "flex",
                "flex-direction": "column",
                "align-items": "center",
                "justify-content": "center",
                "height": "300px",
                "color": "var(--text-muted)",
            },
        ),
        rx.box(
            rx.foreach(AppState.books, book_card),
            class_name="books-grid",
        ),
    )

def books_table() -> rx.Component:
    return rx.cond(
        AppState.books.length() == 0,
        rx.box(
            rx.text("📚", font_size="48px", margin_bottom="12px"),
            rx.text("Aucun livre trouvé dans la bibliothèque."),
            style={
                "display": "flex",
                "flex-direction": "column",
                "align-items": "center",
                "justify-content": "center",
                "height": "300px",
                "color": "var(--text-muted)",
            },
        ),
        rx.box(
            rx.table.root(
                rx.table.header(
                    rx.table.row(
                        rx.table.column_header_cell("Titre"),
                        rx.table.column_header_cell("Auteur"),
                        rx.table.column_header_cell("Éditeur"),
                        rx.table.column_header_cell("Formats"),
                    ),
                ),
                rx.table.body(
                    rx.foreach(
                        AppState.books,
                        lambda book: rx.table.row(
                            rx.table.cell(rx.text(book.title, font_weight="600")),
                            rx.table.cell(book.author),
                            rx.table.cell(rx.cond(book.publisher != "", book.publisher, "-")),
                            rx.table.cell(
                                rx.text(
                                    rx.cond(
                                        book.formats.length() > 0,
                                        book.formats[0].format.upper(),
                                        "EPUB"
                                    ),
                                    class_name="format-badge",
                                ),
                            ),
                            class_name=rx.cond(AppState.selected_book.id == book.id, "selected", ""),
                            on_click=lambda: AppState.select_book(book),
                            cursor="pointer",
                        ),
                    ),
                ),
                class_name="books-table",
            ),
            class_name="books-list",
        ),
    )

def books_view() -> rx.Component:
    return rx.box(
        rx.cond(
            AppState.view_mode == "grid",
            books_grid(),
            books_table(),
        ),
        class_name="books-container",
    )
