CREATE TABLE IF NOT EXISTS audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL, action TEXT NOT NULL, status TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL, actor TEXT, request_id TEXT, client_id TEXT,
    profile_id TEXT, server_id TEXT, session_id TEXT, protocol_version TEXT,
    http_method TEXT, route TEXT, mcp_method TEXT, target TEXT, direction TEXT,
    error_code TEXT, error_message TEXT, detail TEXT, duration_ms INTEGER,
    data_json TEXT, task_id TEXT, related_task_id TEXT, progress_token TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_events_occurred_at ON audit_events (occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_category_action ON audit_events (category, action, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_status ON audit_events (status, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_server_id ON audit_events (server_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_profile_id ON audit_events (profile_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_client_id ON audit_events (client_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_session_id ON audit_events (session_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_task_id ON audit_events (task_id, occurred_at_ms DESC, id DESC);
CREATE TABLE IF NOT EXISTS audit_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1), policy TEXT NOT NULL,
    sweep_interval_secs INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL
);
