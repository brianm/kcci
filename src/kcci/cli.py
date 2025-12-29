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
    query_embedding = embed.embed_query_onnx(query_text)
    results = db.search_semantic(conn, query_embedding, limit)

    if not results:
        click.echo("No results found. Run 'kcci sync' first.")
        return

    for book in results:
        authors = format_authors(book["authors"])
        distance = book.get("distance", "?")
        click.echo(f"[{distance:.3f}] {book['title']} by {authors}")


@cli.command(name="explore")
@click.option("--limit", "-n", default=10, help="Number of results")
def explore(limit: int):
    """Interactive semantic search - load model once, search repeatedly."""
    conn = db.connect()
    db.init_schema(conn)

    click.echo("Loading embedding model...", nl=False)
    tokenizer, model = embed.get_onnx_model()
    click.echo(" ready!")
    click.echo("Enter queries to search, empty line or Ctrl+C to quit.\n")

    import numpy as np

    def encode(query: str) -> list[float]:
        inputs = tokenizer(query, return_tensors="np", padding=True, truncation=True)
        outputs = model(**inputs)
        attention_mask = inputs["attention_mask"]
        token_embs = outputs.last_hidden_state
        input_mask_expanded = np.expand_dims(attention_mask, -1)
        sum_embs = np.sum(token_embs * input_mask_expanded, axis=1)
        sum_mask = np.clip(np.sum(input_mask_expanded, axis=1), a_min=1e-9, a_max=None)
        embedding = (sum_embs / sum_mask)[0]
        # L2 normalize
        norm = np.linalg.norm(embedding)
        if norm > 0:
            embedding = embedding / norm
        return embedding.tolist()

    while True:
        try:
            query = input("search> ").strip()
            if not query:
                break

            query_embedding = encode(query)
            results = db.search_semantic(conn, query_embedding, limit)

            if not results:
                click.echo("No results found.\n")
                continue

            for book in results:
                authors = format_authors(book["authors"])
                distance = book.get("distance", "?")
                click.echo(f"  [{distance:.3f}] {book['title']} by {authors}")
            click.echo()

        except (KeyboardInterrupt, EOFError):
            click.echo("\nBye!")
            break


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


@cli.command()
@click.option("--port", "-p", default=5000, help="Port to run on")
def serve(port: int):
    """Start the web UI."""
    from . import web
    click.echo(f"Starting KCCI web UI at http://localhost:{port}")
    web.run(port=port)


if __name__ == "__main__":
    cli()
