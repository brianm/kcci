"""Extract Kindle library data from Safari webarchive files."""

import json
import plistlib
import re
from pathlib import Path
from typing import Optional


def extract_html_from_webarchive(webarchive_path: Path) -> bytes:
    """Extract the main HTML content from a Safari webarchive file."""
    with open(webarchive_path, 'rb') as f:
        plist = plistlib.load(f)

    main_resource = plist.get('WebMainResource', {})
    return main_resource.get('WebResourceData', b'')


def extract_books_from_html(html: bytes) -> list[dict]:
    """Extract book data from Kindle library HTML."""
    books = []
    seen_asins = set()

    html_str = html.decode('utf-8', errors='replace')

    # Look for the itemViewResponse JSON embedded in the page
    # It's in a script tag with id="itemViewResponse"
    pattern = r'<script[^>]*id="itemViewResponse"[^>]*>(.*?)</script>'
    match = re.search(pattern, html_str, re.DOTALL)

    if match:
        try:
            data = json.loads(match.group(1))
            for item in data.get('itemsList', []):
                asin = item.get('asin')
                if asin and asin not in seen_asins:
                    seen_asins.add(asin)
                    books.append({
                        'asin': asin,
                        'title': item.get('title', ''),
                        'authors': [a.rstrip(':').strip() for a in item.get('authors', [])],
                        'coverUrl': item.get('productUrl', ''),
                        'percentageRead': item.get('percentageRead', 0),
                        'resourceType': item.get('resourceType', 'EBOOK'),
                        'originType': item.get('originType', 'PURCHASE'),
                    })
        except json.JSONDecodeError:
            pass

    # Also try to extract from DOM elements (for lazy-loaded content)
    # Pattern: id="coverContainer-{ASIN}"
    cover_pattern = r'id="coverContainer-([A-Z0-9]+)"'
    for match in re.finditer(cover_pattern, html_str):
        asin = match.group(1)
        if asin in seen_asins:
            continue
        seen_asins.add(asin)

        # Try to find title and author for this ASIN
        title = ''
        authors = []

        title_pattern = rf'id="title-{asin}"[^>]*>.*?<p[^>]*>([^<]+)</p>'
        title_match = re.search(title_pattern, html_str, re.DOTALL)
        if title_match:
            title = title_match.group(1).strip()

        author_pattern = rf'id="author-{asin}"[^>]*>.*?<p[^>]*>([^<]+)</p>'
        author_match = re.search(author_pattern, html_str, re.DOTALL)
        if author_match:
            author_str = author_match.group(1).strip().rstrip(':')
            authors = [a.strip() for a in author_str.split(',')]

        cover_url = ''
        cover_img_pattern = rf'id="cover-{asin}"[^>]*src="([^"]+)"'
        cover_match = re.search(cover_img_pattern, html_str)
        if cover_match:
            cover_url = cover_match.group(1)

        books.append({
            'asin': asin,
            'title': title,
            'authors': authors,
            'coverUrl': cover_url,
            'percentageRead': 0,
            'resourceType': 'EBOOK',
            'originType': 'PURCHASE',
        })

    return books


def parse_webarchive(webarchive_path: Path) -> list[dict]:
    """Parse a Safari webarchive and extract Kindle library books."""
    html = extract_html_from_webarchive(webarchive_path)
    return extract_books_from_html(html)
