import hashlib
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from package_runtime_db import main


class PackageRuntimeDatabaseTest(unittest.TestCase):
    def test_manifest_matches_copied_database(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source.db"
            connection = sqlite3.connect(source)
            connection.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            connection.executemany("INSERT INTO meta VALUES (?, ?)", [("built", "build-1"), ("blob_codec", "zstd")])
            connection.commit()
            connection.close()
            import sys
            old_argv = sys.argv
            try:
                sys.argv = ["package_runtime_db.py", str(source), str(root / "dist"), "--core-ref", "v0.9.11", "--core-sha", "a" * 40]
                main()
            finally:
                sys.argv = old_argv
            database = (root / "dist/haqor.db").read_bytes()
            manifest = json.loads((root / "dist/haqor.db.manifest.json").read_text())
            self.assertEqual(manifest["db_size"], len(database))
            self.assertEqual(manifest["db_sha256"], hashlib.sha256(database).hexdigest())
            self.assertEqual((root / "dist/haqor.db.sha256").read_text(), f"{manifest['db_sha256']}  haqor.db\n")


if __name__ == "__main__":
    unittest.main()
