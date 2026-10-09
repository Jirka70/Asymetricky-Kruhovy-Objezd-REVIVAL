"""Shared connection handling for the local Docker PostgreSQL scripts."""

import argparse
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent


def run(command, **kwargs):
    return subprocess.run(command, check=True, capture_output=True, text=True, **kwargs)


@dataclass(frozen=True)
class Database:
    container: str
    user: str
    name: str

    def execute(self, source, *, maintenance=False):
        result = run(
            [
                "docker", "exec", "-i", self.container,
                "psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1",
                "-v", "VERBOSITY=terse",
                "-v", f"target_db={self.name}",
                "-v", f"target_owner={self.user}",
                "-U", self.user, "-d", "postgres" if maintenance else self.name,
            ],
            input=source,
        )
        return result.stdout.strip()


def connection(description):
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument("--container", help="Docker container; otherwise discover this project's db service")
    parser.add_argument("--database", help="Database name; defaults to POSTGRES_DB in the container")
    args = parser.parse_args()

    if args.container:
        containers = json.loads(run(["docker", "inspect", args.container]).stdout)
    else:
        ids = run([
            "docker", "ps", "-q", "--filter", "label=com.docker.compose.service=db",
        ]).stdout.split()
        containers = json.loads(run(["docker", "inspect", *ids]).stdout) if ids else []
        containers = [
            c for c in containers
            if (directory := (c["Config"].get("Labels") or {}).get(
                "com.docker.compose.project.working_dir"
            )) and Path(directory).resolve() == PROJECT_ROOT
        ]

    if len(containers) != 1:
        parser.error("Expected one running database container for this project; use --container NAME.")
    container = containers[0]
    if not container["State"]["Running"]:
        parser.error("The selected database container is not running.")
    environment = dict(item.split("=", 1) for item in container["Config"].get("Env", []) if "=" in item)
    user = environment.get("POSTGRES_USER", "postgres")
    name = args.database if args.database is not None else environment.get("POSTGRES_DB", user)
    if not name or name in {"postgres", "template0", "template1"}:
        parser.error("Choose an application database, not postgres/template0/template1.")
    return Database(container["Name"].lstrip("/"), user, name)


def main(action):
    try:
        action()
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.stderr.strip() or f"Command failed with exit code {error.returncode}.")
    except OSError as error:
        raise SystemExit(str(error))
