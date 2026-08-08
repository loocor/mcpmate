CREATE TABLE IF NOT EXISTS server_transport (
    server_id TEXT PRIMARY KEY,
    draft_json TEXT NOT NULL CHECK (
        json_valid(draft_json)
        AND json_type(draft_json, '$.kind') IS 'text'
        AND json_extract(draft_json, '$.kind') IN ('stdio', 'http', 'unrecognized')
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS server_config_migration_audit (
    server_id TEXT PRIMARY KEY,
    original_shape_json TEXT NOT NULL CHECK (json_valid(original_shape_json)),
    ignored_field_names_json TEXT NOT NULL CHECK (json_valid(ignored_field_names_json)),
    diagnostic_codes_json TEXT NOT NULL CHECK (json_valid(diagnostic_codes_json)),
    migrated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE
);
