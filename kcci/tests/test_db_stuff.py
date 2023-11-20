import sqlite3
import sqlite_vss # type: ignore

def test_sqlite_vss_loads():
    db = sqlite3.connect(':memory:')
    db.enable_load_extension(True)
    sqlite_vss.load(db)
    db.enable_load_extension(False)
    version, = db.execute('select vss_version()').fetchone()
    assert version == "v0.1.2"
    db.close()

import kcci.db_play as dbp
def test_foo():
    assert dbp.foo() == "hello"