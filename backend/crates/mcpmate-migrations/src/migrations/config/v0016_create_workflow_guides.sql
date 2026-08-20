CREATE TABLE workflow_profile_guides (
    profile_id TEXT PRIMARY KEY,
    guide_revision INTEGER NOT NULL DEFAULT 0,
    markdown TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_guide_steps (
    profile_id TEXT NOT NULL,
    step_key TEXT NOT NULL,
    step_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    PRIMARY KEY (profile_id, step_key),
    UNIQUE (profile_id, step_id),
    UNIQUE (profile_id, ordinal),
    FOREIGN KEY (profile_id, step_id)
        REFERENCES workflow_profile_steps (profile_id, step_id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_capability_aliases (
    profile_id TEXT NOT NULL,
    alias TEXT NOT NULL,
    display_name TEXT NOT NULL,
    ref_id TEXT NOT NULL,
    binding_policy TEXT NOT NULL CHECK (binding_policy IN ('meta_on_demand', 'direct')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (profile_id, alias),
    UNIQUE (profile_id, ref_id),
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_package_files (
    package_file_id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    file_revision INTEGER NOT NULL DEFAULT 0,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    title TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('reference', 'script', 'asset')),
    relative_path TEXT NOT NULL,
    mime_type TEXT,
    extension TEXT,
    file_size INTEGER,
    checksum TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (profile_id, relative_path),
    UNIQUE (profile_id, ordinal),
    UNIQUE (profile_id, package_file_id),
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_external_guides (
    package_file_id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    markdown TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (profile_id, package_file_id)
        REFERENCES workflow_profile_package_files (profile_id, package_file_id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_skill_projections (
    profile_id TEXT PRIMARY KEY,
    input_fingerprint TEXT,
    projected_at TIMESTAMP,
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_guide_step_package_files (
    profile_id TEXT NOT NULL,
    step_key TEXT NOT NULL,
    package_file_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    PRIMARY KEY (profile_id, step_key, ordinal),
    UNIQUE (profile_id, step_key, package_file_id),
    FOREIGN KEY (profile_id, step_key)
        REFERENCES workflow_profile_guide_steps (profile_id, step_key) ON DELETE CASCADE,
    FOREIGN KEY (profile_id, package_file_id)
        REFERENCES workflow_profile_package_files (profile_id, package_file_id) ON DELETE CASCADE
);
