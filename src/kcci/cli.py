import click  # type: ignore
from pathlib import Path
from typing import List

from . import book_finder as bf
from . import db
from . import enrich
from . import embed
from . import sync as sync_module
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
@click.option("--limit", "-n", default=None, type=int, help="Number of books to enrich (default: all)")
@click.option("--delay", "-d", default=1.0, help="Delay between API calls (seconds)")
def enrich_books(limit: int, delay: float):
    """Enrich books with OpenLibrary metadata (via API, slow)."""
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


@cli.command()
@click.argument("webarchive", type=click.Path(exists=True, path_type=Path), required=False)
@click.option("--delay", "-d", default=1.0, help="Delay between API calls (seconds)")
def sync(webarchive: Path, delay: float):
    """Import, enrich, and embed books in one step.

    If WEBARCHIVE is provided, imports books from it first.
    Then enriches any unenriched books via OpenLibrary API.
    Finally generates embeddings for enriched books.

    Safe to interrupt and resume - picks up where it left off.
    """
    conn = db.connect()
    db.init_schema(conn)

    def progress(stage: str, message: str):
        click.echo(f"[{stage}] {message}")

    stats = sync_module.sync(conn, webarchive, delay, progress)

    click.echo("")
    click.echo(f"Done! {stats['imported']} imported, {stats['enriched']} enriched, {stats['embedded']} embedded")

    s = db.get_stats(conn)
    click.echo(f"Total: {s['total_books']} books, {s['enriched']} enriched, {s['with_embeddings']} with embeddings")


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
