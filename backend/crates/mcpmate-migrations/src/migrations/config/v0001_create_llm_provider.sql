CREATE TABLE IF NOT EXISTS llm_provider (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, provider_type TEXT NOT NULL,
    base_url TEXT NOT NULL, model_id TEXT NOT NULL, secret_alias TEXT,
    default_params_json TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
