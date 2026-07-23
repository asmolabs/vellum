from xml.etree import ElementTree as ET
from typing import List
from ..models.book import Book

class OPDSService:
    @staticmethod
    def generate_opds_feed(books: List[Book], base_url: str) -> str:
        """Generates an OPDS 1.2 XML Feed for e-readers and mobile apps."""
        feed = ET.Element("feed", {
            "xmlns": "http://www.w3.org/2005/Atom",
            "xmlns:dc": "http://purl.org/dc/terms/",
            "xmlns:opds": "http://opds-spec.org/2010/catalog"
        })

        id_elem = ET.SubElement(feed, "id")
        id_elem.text = "urn:uuid:vellum-library-catalog"

        title_elem = ET.SubElement(feed, "title")
        title_elem.text = "Bibliothèque Vellum"

        updated_elem = ET.SubElement(feed, "updated")
        import datetime
        updated_elem.text = datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")

        # Self link
        ET.SubElement(feed, "link", {
            "rel": "self",
            "href": f"{base_url}/opds/v1/catalog.xml",
            "type": "application/atom+xml;profile=opds-catalog;kind=navigation"
        })

        for book in books:
            entry = ET.SubElement(feed, "entry")
            
            e_id = ET.SubElement(entry, "id")
            e_id.text = f"urn:uuid:{book.id}"
            
            e_title = ET.SubElement(entry, "title")
            e_title.text = book.title
            
            if book.author:
                author_elem = ET.SubElement(entry, "author")
                a_name = ET.SubElement(author_elem, "name")
                a_name.text = book.author
                
            if book.description:
                content_elem = ET.SubElement(entry, "content", {"type": "html"})
                content_elem.text = book.description

            if book.coverPath:
                ET.SubElement(entry, "link", {
                    "rel": "http://opds-spec.org/image",
                    "href": f"{base_url}/books/{book.id}/cover",
                    "type": "image/jpeg"
                })

            for fmt in book.formats:
                mime_type = "application/epub+zip"
                if fmt.format.lower() == "mobi":
                    mime_type = "application/x-mobipocket-ebook"
                elif fmt.format.lower() == "azw3":
                    mime_type = "application/vnd.amazon.ebook"

                ET.SubElement(entry, "link", {
                    "rel": "http://opds-spec.org/acquisition",
                    "href": f"{base_url}/books/{book.id}/file/{fmt.format}",
                    "type": mime_type,
                    "title": f"Télécharger {fmt.format.upper()}"
                })

        return '<?xml version="1.0" encoding="utf-8"?>\n' + ET.tostring(feed, encoding="utf-8").decode("utf-8")
