import reflex as rx
from ..state import AppState

def reader_overlay() -> rx.Component:
    book = AppState.reading_book
    book_id = book.get("id", "")
    epub_url = f"{AppState.api_url}/books/{book_id}/file/epub"

    # HTML code for embedded epub reader iframe
    reader_html_doc = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/jszip/3.1.5/jszip.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/epubjs@0.3.93/dist/epub.min.js"></script>
    <style>
        body {{ margin: 0; padding: 0; background: #121212; overflow: hidden; font-family: sans-serif; height: 100vh; display: flex; align-items: center; justify-content: center; }}
        #area {{ width: 85%; height: 90%; background: #ffffff; box-shadow: 0 4px 20px rgba(0,0,0,0.5); border-radius: 4px; overflow: hidden; }}
        .nav-btn {{ position: absolute; top: 50%; transform: translateY(-50%); background: rgba(255,255,255,0.1); border: none; color: white; width: 44px; height: 44px; border-radius: 22px; cursor: pointer; font-size: 18px; display: flex; align-items: center; justify-content: center; z-index: 10; }}
        .nav-btn:hover {{ background: rgba(255,255,255,0.25); }}
        .prev {{ left: 15px; }}
        .next {{ right: 15px; }}
    </style>
</head>
<body>
    <button class="nav-btn prev" onclick="if(rendition) rendition.prev()">◀</button>
    <div id="area"></div>
    <button class="nav-btn next" onclick="if(rendition) rendition.next()">▶</button>
    <script>
        var book = ePub("{epub_url}");
        var rendition = book.renderTo("area", {{ width: "100%", height: "100%", flow: "paginated" }});
        rendition.display();
    </script>
</body>
</html>"""

    doc_escaped = reader_html_doc.replace('"', '&quot;')

    return rx.cond(
        AppState.is_reading,
        rx.box(
            rx.vstack(
                # Reader Header
                rx.hstack(
                    rx.heading(book.get("title", "Lecture"), size="3", font_weight="500"),
                    rx.hstack(
                        rx.button("A-", on_click=AppState.decrease_font_size, variant="soft", size="1", cursor="pointer"),
                        rx.text(f"{AppState.font_size}%", font_size="13px"),
                        rx.button("A+", on_click=AppState.increase_font_size, variant="soft", size="1", cursor="pointer"),
                        spacing="2",
                        align="center",
                    ),
                    rx.button("Fermer la lecture", on_click=AppState.close_reader, variant="outline", size="2", cursor="pointer"),
                    justify="between",
                    align="center",
                    width="100%",
                    height="50px",
                    padding_x="4",
                    background="#1a1a1a",
                    border_bottom="1px solid rgba(255, 255, 255, 0.1)",
                ),
                # Reader Viewer Frame
                rx.html(
                    f'<iframe srcdoc="{doc_escaped}" style="width:100%; height:calc(100vh - 50px); border:none;"></iframe>',
                    width="100%",
                    flex="1",
                ),
                spacing="0",
                width="100%",
                height="100vh",
            ),
            position="fixed",
            top="0",
            left="0",
            right="0",
            bottom="0",
            background="#121212",
            z_index="200",
            width="100vw",
            height="100vh",
        ),
    )
