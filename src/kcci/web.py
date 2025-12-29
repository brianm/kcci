"""Flask web UI for KCCI."""

import json
import markdown
from markupsafe import Markup
from flask import Flask, render_template, request

from . import db
from . import embed

app = Flask(__name__)


def format_authors(authors_json: str) -> str:
    """Format authors JSON array as readable string."""
    authors = json.loads(authors_json)
    return ", ".join(authors)


def render_markdown(text: str) -> Markup:
    """Render markdown to HTML."""
    if not text:
        return Markup("")
    html = markdown.markdown(text)
    return Markup(html)


# Register template filters
app.jinja_env.filters["authors"] = format_authors
app.jinja_env.filters["markdown"] = render_markdown


@app.route("/")
def index():
    """Dashboard with stats and search."""
    conn = db.connect()
    db.init_schema(conn)
    stats = db.get_stats(conn)
    return render_template("index.html", stats=stats)


@app.route("/search")
def search():
    """Search endpoint - returns partial HTML for HTMX."""
    q = request.args.get("q", "").strip()
    mode = request.args.get("mode", "semantic")
    limit = int(request.args.get("limit", 20))

    if not q:
        return render_template("partials/results.html", books=[], query="")

    conn = db.connect()
    db.init_schema(conn)

    if mode == "semantic":
        query_embedding = embed.embed_query_onnx(q)
        books = db.search_semantic(conn, query_embedding, limit)
    else:
        books = db.search_fts(conn, q, limit)

    return render_template("partials/results.html", books=books, query=q, mode=mode)


@app.route("/books")
def books():
    """Browse all books with pagination."""
    page = int(request.args.get("page", 1))
    per_page = 50
    offset = (page - 1) * per_page

    conn = db.connect()
    db.init_schema(conn)
    stats = db.get_stats(conn)
    all_books = db.get_all_books(conn, limit=per_page, offset=offset)

    total_pages = (stats["total_books"] + per_page - 1) // per_page

    return render_template(
        "books.html",
        books=all_books,
        page=page,
        total_pages=total_pages,
        stats=stats,
    )


@app.route("/book/<asin>")
def book_detail(asin):
    """Single book view."""
    conn = db.connect()
    db.init_schema(conn)
    book = db.get_book_by_asin(conn, asin)

    if not book:
        return "Book not found", 404

    # Parse subjects JSON
    subjects = []
    if book.get("subjects"):
        subjects = json.loads(book["subjects"])

    return render_template("book.html", book=book, subjects=subjects)


@app.route("/book/<asin>/detail")
def book_detail_partial(asin):
    """Expanded book card partial for HTMX."""
    score = request.args.get("score")

    conn = db.connect()
    db.init_schema(conn)
    book = db.get_book_by_asin(conn, asin)

    if not book:
        return "Book not found", 404

    subjects = []
    if book.get("subjects"):
        subjects = json.loads(book["subjects"])

    return render_template("partials/book_expanded.html", book=book, subjects=subjects, score=score)


@app.route("/book/<asin>/collapse")
def book_collapse_partial(asin):
    """Collapsed book card partial for HTMX."""
    score = request.args.get("score")

    conn = db.connect()
    db.init_schema(conn)
    book = db.get_book_by_asin(conn, asin)

    if not book:
        return "Book not found", 404

    return render_template("partials/book_collapsed.html", book=book, score=score)


def run(port: int = 5000, debug: bool = True):
    """Run the Flask development server."""
    app.run(port=port, debug=debug)
