"""Flask web UI for KCCI."""

import json
import tempfile
from pathlib import Path

import markdown
from markupsafe import Markup
from flask import Flask, render_template, request, Response

from . import db
from . import embed
from . import sync as sync_module

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


@app.route("/import")
def import_page():
    """Import page with instructions and file upload."""
    conn = db.connect()
    db.init_schema(conn)
    stats = db.get_stats(conn)
    return render_template("import.html", stats=stats)


@app.route("/search")
def search():
    """Search endpoint - returns partial HTML for HTMX."""
    q = request.args.get("q", "").strip()
    mode = request.args.get("mode", "semantic")
    limit = int(request.args.get("limit", 100))

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


@app.route("/sync", methods=["POST"])
def sync_upload():
    """Handle webarchive upload and run sync with SSE progress."""
    import queue
    import threading

    file = request.files.get("file")
    if not file:
        return Response("No file provided", status=400)

    if not file.filename.endswith(".webarchive"):
        return Response("File must be a .webarchive file", status=400)

    # Save uploaded file to temp location
    with tempfile.NamedTemporaryFile(delete=False, suffix=".webarchive") as tmp:
        file.save(tmp.name)
        temp_path = Path(tmp.name)

    # Queue for passing events from sync thread to generator
    event_queue = queue.Queue()

    def run_sync():
        try:
            conn = db.connect()
            db.init_schema(conn)

            def progress_callback(stage, message, current=None, total=None):
                event_queue.put({
                    "stage": stage,
                    "message": message,
                    "current": current,
                    "total": total,
                })

            stats = sync_module.sync(
                conn=conn,
                webarchive_path=temp_path,
                progress_callback=progress_callback,
            )

            # Send completion event
            event_queue.put({"stage": "complete", "stats": stats})

        except Exception as e:
            event_queue.put({"stage": "error", "message": str(e)})
        finally:
            event_queue.put(None)  # Signal end of stream
            temp_path.unlink(missing_ok=True)

    # Start sync in background thread
    thread = threading.Thread(target=run_sync)
    thread.start()

    def generate():
        while True:
            event = event_queue.get()
            if event is None:
                break
            yield f"data: {json.dumps(event)}\n\n"

    return Response(generate(), mimetype="text/event-stream")


def run(port: int = 0):
    """Run the web server. Port 0 means pick a random available port."""
    import socket
    from waitress import serve

    if port == 0:
        # Find a random available port
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]

    print(f"http://localhost:{port}", flush=True)
    serve(app, host="127.0.0.1", port=port)
