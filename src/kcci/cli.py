import json

import click  # type: ignore
from pathlib import Path

from . import db
from . import embed
from . import sync as sync_module


def format_authors(authors_json: str) -> str:
    """Format authors JSON array as readable string."""
    authors = json.loads(authors_json)
    return ", ".join(authors)


@click.group()
def cli():
    """KCCI - Keith's Card Catalog Index"""
    pass


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
        authors = format_authors(book["authors"])
        click.echo(f"{book['title']} by {authors}")


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
        click.echo("No results found. Run 'kcci sync' first.")
        return

    for book in results:
        authors = format_authors(book["authors"])
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


if __name__ == "__main__":
    cli()
