-- Device-local persistence. This table is not itself replicated.
-- `local_ref` is resolved only by the platform that created it.
CREATE TABLE IF NOT EXISTS ftnl_upload_jobs (
  id TEXT PRIMARY KEY NOT NULL,
  tunnel_id TEXT NOT NULL,
  file_id TEXT,
  name TEXT NOT NULL,
  media_type TEXT NOT NULL,
  size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
  bytes_transferred INTEGER NOT NULL DEFAULT 0 CHECK (bytes_transferred >= 0),
  status TEXT NOT NULL,
  attempt INTEGER NOT NULL DEFAULT 0,
  reason_code TEXT,
  updated_at_hlc TEXT NOT NULL,
  synced_at_hlc TEXT,
  local_ref TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS ftnl_upload_jobs_pending
  ON ftnl_upload_jobs(status, created_at_ms)
  WHERE status IN ('queued', 'declaring', 'uploading', 'paused', 'failed');
