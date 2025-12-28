import sqlite3
import sqlite_vec  # type: ignore
import kcci.db_play as dbp


def test_sqlite_vec_loads():
    db = sqlite3.connect(":memory:")
    db.enable_load_extension(True)
    sqlite_vec.load(db)
    db.enable_load_extension(False)
    (version,) = db.execute("select vec_version()").fetchone()
    assert version.startswith("v0.1")
    db.close()


def test_foo():
    assert dbp.foo() == "hello"


def woof() -> set[str]:
    return {"woof"}


def test_woof():
    assert woof() == {"woof"}
