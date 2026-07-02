import os
import zipfile
import xml.etree.ElementTree as ET
from urllib.parse import unquote

class BookMetadata:
    def __init__(self, title="Untitled", author="Unknown", publisher=None, publish_date=None, description=None, tags=None, cover_data=None, cover_ext=None):
        self.title = title
        self.author = author
        self.publisher = publisher
        self.publish_date = publish_date
        self.description = description
        self.tags = tags or []
        self.cover_data = cover_data
        self.cover_ext = cover_ext

def parse_metadata(filepath: str) -> BookMetadata:
    ext = os.path.splitext(filepath)[1].lower()
    
    if ext == ".epub":
        try:
            return parse_epub(filepath)
        except Exception as e:
            print(f"Error parsing EPUB {filepath}: {e}")
            
    # Fallback for mobi, azw3 or failed epub parse
    filename = os.path.splitext(os.path.basename(filepath))[0]
    parts = filename.split(" - ")
    if len(parts) > 1:
        author = parts[0].strip()
        title = " - ".join(parts[1:]).strip()
    else:
        author = "Unknown"
        title = filename.strip()
        
    return BookMetadata(title=title, author=author)

def parse_epub(filepath: str) -> BookMetadata:
    with zipfile.ZipFile(filepath, 'r') as epub:
        # 1. Read container.xml to find the OPF file path
        try:
            container_xml = epub.read('META-INF/container.xml')
        except KeyError:
            raise Exception("Invalid EPUB: META-INF/container.xml not found")
            
        root = ET.fromstring(container_xml)
        # Handle namespaces
        ns = {'ns': 'urn:oasis:names:tc:opendocument:xmlns:container'}
        rootfile_el = root.find('.//ns:rootfile', ns)
        if rootfile_el is None:
            # Try without namespace
            rootfile_el = root.find('.//rootfile')
            
        if rootfile_el is None or 'full-path' not in rootfile_el.attrib:
            raise Exception("Invalid EPUB: rootfile path not found")
            
        opf_path = rootfile_el.attrib['full-path']
        
        # 2. Read OPF file
        try:
            opf_xml = epub.read(opf_path)
        except KeyError:
            raise Exception(f"Invalid EPUB: OPF file not found at {opf_path}")
            
        opf_dir = os.path.dirname(opf_path)
        opf_root = ET.fromstring(opf_xml)
        
        # Namespaces in OPF
        opf_ns = {
            'opf': 'http://www.idpf.org/2007/opf',
            'dc': 'http://purl.org/dc/elements/1.1/'
        }
        
        metadata_el = opf_root.find('.//opf:metadata', opf_ns)
        if metadata_el is None:
            metadata_el = opf_root.find('.//metadata')
            
        if metadata_el is None:
            raise Exception("Invalid EPUB: metadata block not found")
            
        # Helper to find dc elements
        def get_dc_value(tag, multiple=False):
            elements = metadata_el.findall(f'.//dc:{tag}', opf_ns)
            if not elements:
                elements = metadata_el.findall(f'.//{tag}')
            if not elements:
                return [] if multiple else None
            
            vals = [el.text for el in elements if el.text]
            if multiple:
                return vals
            return vals[0] if vals else None

        title = get_dc_value('title') or "Untitled"
        author = get_dc_value('creator') or "Unknown"
        publisher = get_dc_value('publisher')
        publish_date = get_dc_value('date')
        description = get_dc_value('description')
        tags = get_dc_value('subject', multiple=True)
        
        # 3. Find cover image
        cover_data = None
        cover_ext = None
        
        # Try to find cover via meta element
        cover_id = None
        meta_elements = metadata_el.findall('.//opf:meta', opf_ns) or metadata_el.findall('.//meta')
        for meta in meta_elements:
            if meta.attrib.get('name') == 'cover':
                cover_id = meta.attrib.get('content')
                break
                
        # Find item in manifest
        manifest_el = opf_root.find('.//opf:manifest', opf_ns) or opf_root.find('.//manifest')
        cover_href = None
        
        if manifest_el is not None:
            items = manifest_el.findall('.//opf:item', opf_ns) or manifest_el.findall('.//item')
            
            # If we found a cover_id, search for that item
            if cover_id:
                for item in items:
                    if item.attrib.get('id') == cover_id:
                        cover_href = item.attrib.get('href')
                        break
            
            # Fallback: search for items with 'cover' in their id or properties
            if not cover_href:
                for item in items:
                    iid = item.attrib.get('id', '').lower()
                    properties = item.attrib.get('properties', '').lower()
                    href = item.attrib.get('href', '').lower()
                    if 'cover' in iid or 'cover' in properties or 'cover' in href:
                        if item.attrib.get('media-type', '').startswith('image/'):
                            cover_href = item.attrib.get('href')
                            break
                            
            if cover_href:
                # Resolve cover_href relative to OPF path
                cover_href = unquote(cover_href)
                cover_zip_path = os.path.normpath(os.path.join(opf_dir, cover_href)) if opf_dir else cover_href
                # zip paths always use forward slashes
                cover_zip_path = cover_zip_path.replace('\\', '/')
                try:
                    cover_data = epub.read(cover_zip_path)
                    cover_ext = os.path.splitext(cover_href)[1].lower() or '.jpg'
                except KeyError:
                    # Try direct match without normalization just in case
                    try:
                        cover_data = epub.read(cover_href)
                        cover_ext = os.path.splitext(cover_href)[1].lower() or '.jpg'
                    except KeyError:
                        pass
                        
        return BookMetadata(
            title=title,
            author=author,
            publisher=publisher,
            publish_date=publish_date,
            description=description,
            tags=tags,
            cover_data=cover_data,
            cover_ext=cover_ext
        )
