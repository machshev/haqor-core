#!/usr/bin/env python3
"""Package the validated shipping SQLite database and provenance files."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import sqlite3
from pathlib import Path

DB = "haqor.db"
MANIFEST = "haqor.db.manifest.json"
CHECKSUM = "haqor.db.sha256"


def metadata(path: Path) -> dict[str, str]:
    connection = sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True)
    try:
        values = dict(connection.execute(
            "SELECT key, value FROM meta WHERE key IN ('built', 'blob_codec')"
        ).fetchall())
    finally:
        connection.close()
    if set(values) != {"built", "blob_codec"} or any(
        not isinstance(value, str) or not value for value in values.values()
    ):
        raise ValueError("database meta built/blob_codec must be non-empty text")
    return values


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("database", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--core-ref", required=True)
    parser.add_argument("--core-sha", required=True)
    parser.add_argument("--blob-codec", choices=("none", "zstd"), default="zstd")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    destination = args.output / DB
    shutil.copyfile(args.database, destination)
    values = metadata(destination)
    if values["blob_codec"] != args.blob_codec:
        raise ValueError(f"database codec {values['blob_codec']} != {args.blob_codec}")
    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    manifest = {
        "schema_version": 1,
        "core_ref": args.core_ref,
        "core_sha": args.core_sha,
        "blob_codec": args.blob_codec,
        "db_built": values["built"],
        "db_size": destination.stat().st_size,
        "db_sha256": digest,
    }
    (args.output / MANIFEST).write_text(json.dumps(manifest, sort_keys=True) + "\n")
    (args.output / CHECKSUM).write_text(f"{digest}  {DB}\n")


if __name__ == "__main__":
    main()
