#!/usr/bin/env python3
"""Drop the project's database, including all its data and active connections."""

from db_common import connection, main


def drop():
    db = connection(__doc__)
    print(f"Dropping database {db.name} in {db.container}...", flush=True)
    db.execute('DROP DATABASE IF EXISTS :"target_db" WITH (FORCE);', maintenance=True)
    print(f"Database {db.name} dropped.")


if __name__ == "__main__":
    main(drop)
