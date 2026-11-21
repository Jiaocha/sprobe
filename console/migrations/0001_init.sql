-- Migration 0001: Init schema for Sprobe Console D1 database

CREATE TABLE IF NOT EXISTS agents (
  agent_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  os TEXT NOT NULL,
  arch TEXT NOT NULL,
  labels_json TEXT NOT NULL DEFAULT '{}',
  first_seen TEXT NOT NULL,
  last_seen TEXT NOT NULL,
  last_push_at TEXT,
  last_heartbeat_at TEXT,
  wal_snapshots INTEGER NOT NULL DEFAULT 0,
  last_instance_id TEXT,
  last_sequence INTEGER NOT NULL DEFAULT -1,
  last_collected_at TEXT,
  health_status TEXT NOT NULL DEFAULT 'unknown',
  health_json TEXT,
  capabilities_json TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS snapshots_latest (
  agent_id TEXT PRIMARY KEY,
  instance_id TEXT NOT NULL,
  sequence INTEGER NOT NULL,
  collected_at TEXT NOT NULL,
  snapshot_json TEXT NOT NULL,
  received_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS metrics_1m (
  agent_id TEXT NOT NULL,
  bucket_ts TEXT NOT NULL,
  cpu_usage_ratio REAL,
  cpu_iowait_ratio REAL,
  cpu_steal_ratio REAL,
  memory_usage_ratio REAL,
  memory_used_bytes INTEGER,
  memory_cached_bytes INTEGER,
  swap_used_bytes INTEGER,
  load_average_1m REAL,
  receive_bytes_per_second REAL,
  transmit_bytes_per_second REAL,
  health_status TEXT,
  PRIMARY KEY (agent_id, bucket_ts)
);

CREATE TABLE IF NOT EXISTS ingest_batches (
  agent_id TEXT NOT NULL,
  instance_id TEXT NOT NULL,
  batch_id TEXT NOT NULL,
  first_sequence INTEGER NOT NULL,
  last_sequence INTEGER NOT NULL,
  received_at TEXT NOT NULL,
  status TEXT NOT NULL,
  PRIMARY KEY (agent_id, instance_id, batch_id)
);

CREATE TABLE IF NOT EXISTS ingest_tokens (
  token_hash TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  expires_at TEXT,
  revoked INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ingest_tokens_agent ON ingest_tokens(agent_id);
