CREATE TABLE IF NOT EXISTS server_config (
    id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE,
    server_type TEXT NOT NULL CHECK (server_type IN ('stdio', 'sse', 'streamable_http')),
    command TEXT, url TEXT, source TEXT, enabled BOOLEAN NOT NULL DEFAULT 1,
    unify_direct_exposure_eligible BOOLEAN NOT NULL DEFAULT 0,
    pending_import BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS server_args (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL,
    arg_index INTEGER NOT NULL, arg_value TEXT NOT NULL,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, arg_index)
);
CREATE TABLE IF NOT EXISTS server_env (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL,
    env_key TEXT NOT NULL, env_value TEXT NOT NULL,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, env_key)
);
CREATE TABLE IF NOT EXISTS server_headers (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, header_key TEXT NOT NULL,
    header_value TEXT NOT NULL, FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, header_key)
);
CREATE TABLE IF NOT EXISTS server_meta (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL,
    author TEXT, category TEXT, description TEXT, extras_json TEXT, icons_json TEXT,
    protocol_version TEXT, rating INTEGER, recommended_scenario TEXT, registry_meta_json TEXT,
    registry_version TEXT, repository TEXT, upstream_name TEXT, upstream_title TEXT,
    server_version TEXT, website TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE, UNIQUE(server_id)
);
CREATE TABLE IF NOT EXISTS server_namespace_issue (
    server_id TEXT PRIMARY KEY, issue_kind TEXT NOT NULL, capability_kind TEXT,
    external_identifier TEXT, upstream_value TEXT, conflicting_server_id TEXT,
    conflicting_upstream_value TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    FOREIGN KEY (conflicting_server_id) REFERENCES server_config (id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS server_oauth_config (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL UNIQUE, authorization_endpoint TEXT NOT NULL,
    token_endpoint TEXT NOT NULL, client_id TEXT NOT NULL, client_secret TEXT, scopes TEXT,
    redirect_uri TEXT NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS server_oauth_tokens (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL UNIQUE, access_token TEXT NOT NULL,
    refresh_token TEXT, token_type TEXT NOT NULL DEFAULT 'bearer', expires_at TEXT, scope TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE
);
