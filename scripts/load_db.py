#!/usr/bin/env python3
"""Create the project's database if needed and load all SQL imports in order."""

from db_common import PROJECT_ROOT, connection, main


IMPORTS = (
    "scripts/sql/insert_zsj.sql",
    "scripts/sql/insert_skoly.sql",
    "scripts/insert_obory_nabidka_oboru.sql",
    "scripts/insert_demografie_zsj.sql",
)


def load():
    db = connection(__doc__)
    # Read all inputs before creating or modifying the database.
    sources = [(path, (PROJECT_ROOT / path).read_text(encoding="utf-8")) for path in IMPORTS]
    print(f"Loading database {db.name} in {db.container}...", flush=True)
    db.execute(r"""
SELECT format('CREATE DATABASE %I OWNER %I TEMPLATE template0 ENCODING ''UTF8''',
              :'target_db', :'target_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = :'target_db')
\gexec
""", maintenance=True)
    for path, source in sources:
        print(f"Importing {path}...", flush=True)
        db.execute(source)
    print(f"Database {db.name} loaded.")


if __name__ == "__main__":
    main(load)
