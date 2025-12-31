use rusqlite_migration::{Migrations, M};

/// Define all database migrations.
/// Each migration runs in a transaction and updates PRAGMA user_version.
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        // v1: Initial schema from original schema.sql
        M::up(include_str!("migrations/v001_initial.sql")),
        // Future migrations:
        // M::up("ALTER TABLE books ADD COLUMN new_field TEXT;"),
    ])
}
