#!/bin/sh
set -eu

# Compose waits for PostgreSQL health before starting this container.
# Stop on migration failure instead of serving against an outdated schema.
if [ "${1:-}" = "serve" ]; then
    /usr/local/bin/obor-backend migrate
fi

exec /usr/local/bin/obor-backend "$@"
