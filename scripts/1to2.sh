#!/usr/bin/bash
# Migrate compdb store format from rua 1.x to 2.x.
set -euo pipefail

sqlite3 compdb.store <<EOF
CREATE TABLE IF NOT EXISTS compdbs (
  generation INTEGER PRIMARY KEY AUTOINCREMENT,
  branch TEXT NOT NULL,
  revision TEXT NOT NULL,
  target TEXT NOT NULL,
  timestamp INTEGER NOT NULL,
  compdb BLOB NOT NULL,
  remark TEXT
);
CREATE TABLE IF NOT EXISTS history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  generation INTEGER
);
attach database 'compdbs.db3' as source_db;
INSERT INTO compdbs (branch, revision, target, timestamp, compdb, remark) SELECT branch, revision, target, timestamp, compdb, remark FROM source_db.compdbs;
EOF
