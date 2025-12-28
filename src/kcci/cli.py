import click  # type: ignore
from pathlib import Path
from typing import List

from . import book_finder as bf
from . import db
from . import enrich
from . import embed
from . import ol_dump
from . import webarchive


@click.group()
def cli():
    """KCCI - Keith's Card Catalog Index"""
    pass


@cli.command(name="import")
@click.argument("json_file", type=click.Path(exists=True, path_type=Path))
def import_books(json_file: Path):
    """Import books from Kindle library JSON export."""
    conn = db.connect()
    db.init_schema(conn)
    count = db.import_kindle_json(conn, json_file)
    db.rebuild_fts(conn)
    stats = db.get_stats(conn)
    click.echo(f"Imported {count} new books. Total: {stats['total_books']}")


@cli.command(name="import-webarchive")
@click.argument("webarchive_file", type=click.Path(exists=True, path_type=Path))
def import_webarchive(webarchive_file: Path):
    """Import books from Safari webarchive of Kindle library page."""
    conn = db.connect()
    db.init_schema(conn)

    click.echo(f"Extracting books from {webarchive_file}...")
    books = webarchive.parse_webarchive(webarchive_file)
    click.echo(f"Found {len(books)} books in webarchive")

    if not books:
        click.echo("No books found.")
        return

    # Save to temp JSON and import
    import json
    import tempfile
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(books, f)
        temp_path = Path(f.name)

    count = db.import_kindle_json(conn, temp_path)
    temp_path.unlink()

    db.rebuild_fts(conn)
    stats = db.get_stats(conn)
    click.echo(f"Imported {count} new books. Total: {stats['total_books']}")


@cli.command()
def stats():
    """Show database statistics."""
    conn = db.connect()
    db.init_schema(conn)
    s = db.get_stats(conn)
    click.echo(f"Total books:     {s['total_books']}")
    click.echo(f"Enriched:        {s['enriched']}")
    click.echo(f"With embeddings: {s['with_embeddings']}")


@cli.command()
@click.argument("query", nargs=-1, required=True)
@click.option("--limit", "-n", default=10, help="Number of results")
def search(query: tuple[str, ...], limit: int):
    """Full-text search for books."""
    conn = db.connect()
    db.init_schema(conn)
    results = db.search_fts(conn, " ".join(query), limit)
    if not results:
        click.echo("No results found.")
        return
    for book in results:
        authors = book["authors"]
        click.echo(f"{book['title']} by {authors}")


@cli.command(name="enrich")
@click.option("--limit", "-n", default=10, help="Number of books to enrich")
@click.option("--delay", "-d", default=1.0, help="Delay between API calls (seconds)")
def enrich_books(limit: int, delay: float):
    """Enrich books with OpenLibrary metadata."""
    conn = db.connect()
    db.init_schema(conn)

    def progress(current, total, title):
        click.echo(f"[{current}/{total}] {title[:50]}")

    success, total = enrich.enrich_batch(conn, limit, delay, progress)
    click.echo(f"Enriched {success}/{total} books with descriptions")
    s = db.get_stats(conn)
    click.echo(f"Total enriched: {s['enriched']}/{s['total_books']}")


@cli.command(name="embed")
@click.option("--limit", "-n", default=100, help="Number of books to embed")
def embed_books(limit: int):
    """Generate embeddings for enriched books."""
    conn = db.connect()
    db.init_schema(conn)

    def progress(current, total, title):
        click.echo(f"[{current}/{total}] {title[:50]}")

    count = embed.embed_batch(conn, limit, progress)
    click.echo(f"Generated {count} embeddings")
    s = db.get_stats(conn)
    click.echo(f"Total with embeddings: {s['with_embeddings']}/{s['total_books']}")


@cli.command(name="semantic")
@click.argument("query", nargs=-1, required=True)
@click.option("--limit", "-n", default=10, help="Number of results")
def semantic_search(query: tuple[str, ...], limit: int):
    """Semantic search using embeddings."""
    conn = db.connect()
    db.init_schema(conn)

    query_text = " ".join(query)
    query_embedding = embed.embed_query(query_text)
    results = db.search_semantic(conn, query_embedding, limit)

    if not results:
        click.echo("No results found. Run 'kcci embed' first to generate embeddings.")
        return

    for book in results:
        authors = book["authors"]
        distance = book.get("distance", "?")
        click.echo(f"[{distance:.3f}] {book['title']} by {authors}")


@cli.command(name="import-ol-works")
@click.argument("dump_file", type=click.Path(exists=True, path_type=Path))
def import_ol_works(dump_file: Path):
    """Import OpenLibrary works dump for fast enrichment.

    Download from: https://openlibrary.org/data/ol_dump_works_latest.txt.gz
    """
    conn = db.connect()
    db.init_schema(conn)

    last_count = [0]
    def progress(count):
        if count - last_count[0] >= 100000:
            click.echo(f"  {count:,} works...")
            last_count[0] = count

    click.echo(f"Importing works from {dump_file}...")
    count = ol_dump.import_works_dump(conn, dump_file, progress)
    click.echo(f"Imported {count:,} works")

    stats = ol_dump.get_dump_stats(conn)
    click.echo(f"Works with descriptions: {stats['works_with_descriptions']:,}")


@cli.command(name="import-ol-authors")
@click.argument("dump_file", type=click.Path(exists=True, path_type=Path))
def import_ol_authors(dump_file: Path):
    """Import OpenLibrary authors dump for author matching.

    Download from: https://openlibrary.org/data/ol_dump_authors_latest.txt.gz
    """
    conn = db.connect()
    db.init_schema(conn)

    last_count = [0]
    def progress(count):
        if count - last_count[0] >= 100000:
            click.echo(f"  {count:,} authors...")
            last_count[0] = count

    click.echo(f"Importing authors from {dump_file}...")
    count = ol_dump.import_authors_dump(conn, dump_file, progress)
    click.echo(f"Imported {count:,} authors")


@cli.command(name="enrich-dump")
@click.option("--limit", "-n", default=None, type=int, help="Number of books to enrich (default: all)")
def enrich_from_dump(limit: int):
    """Enrich books from local OpenLibrary dump (fast, no API calls).

    Requires importing dumps first with import-ol-works and import-ol-authors.
    """
    conn = db.connect()
    db.init_schema(conn)

    stats = ol_dump.get_dump_stats(conn)
    if stats["works"] == 0:
        click.echo("No OpenLibrary dump imported. Run 'kcci import-ol-works' first.")
        click.echo("Download: wget https://openlibrary.org/data/ol_dump_works_latest.txt.gz")
        return

    def progress(current, total, title):
        click.echo(f"[{current}/{total}] {title[:50]}")

    success, total = ol_dump.enrich_from_dump(conn, limit, progress)
    click.echo(f"Enriched {success}/{total} books with descriptions")
    s = db.get_stats(conn)
    click.echo(f"Total enriched: {s['enriched']}/{s['total_books']}")


@cli.command(name="ol-stats")
def ol_stats():
    """Show OpenLibrary dump statistics."""
    conn = db.connect()
    stats = ol_dump.get_dump_stats(conn)
    click.echo(f"Works:             {stats['works']:,}")
    click.echo(f"With descriptions: {stats['works_with_descriptions']:,}")
    click.echo(f"Authors:           {stats['authors']:,}")


@cli.command(name="basics")
@click.option("--num", "-n", default=1)
@click.argument("query", nargs=-1)
def basics(num: int, query: List[str]):
    from . import ss_play as ss  # only import if needed, long load time

    ss.play(num, " ".join(query))


@cli.command()
@click.option("--author", "-a", multiple=True)
@click.option("--title", "-t")
def ol(title: str, author: List[str]):
    book = bf.openlibrary_lookup(title, author)
    click.echo(f"{book.title} by {', '.join(book.authors)}")
    click.echo("")
    click.echo(book.description)


@cli.command()
@click.option("--author", "-a", multiple=True)
@click.option("--title", "-t")
def loc(title: str, author: List[str]):
    book = bf.loc_lookup(title, author)
    click.echo(f"{book.title} by {', '.join(book.authors)}")
    click.echo("")
    click.echo(book.description)


if __name__ == "__main__":
    cli()
