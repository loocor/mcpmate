ALTER TABLE workflow_profile_steps ADD COLUMN step_id TEXT;

UPDATE workflow_profile_steps
SET step_id = lower(hex(randomblob(16)))
WHERE step_id IS NULL;

CREATE UNIQUE INDEX idx_workflow_profile_steps_step_id
ON workflow_profile_steps (profile_id, step_id);

CREATE TABLE workflow_profile_skills (
    profile_id TEXT PRIMARY KEY,
    skill_name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_material_libraries (
    profile_id TEXT PRIMARY KEY,
    materials_revision INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_materials (
    material_id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    material_revision INTEGER NOT NULL DEFAULT 0,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    title TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('external_url', 'uploaded_file', 'markdown_file')),
    external_url TEXT,
    relative_path TEXT,
    original_filename TEXT,
    file_size INTEGER,
    checksum TEXT,
    markdown_content TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
        (kind = 'external_url' AND external_url IS NOT NULL AND relative_path IS NULL)
        OR (kind IN ('uploaded_file', 'markdown_file') AND external_url IS NULL AND relative_path IS NOT NULL)
    ),
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE INDEX idx_workflow_profile_materials_profile
ON workflow_profile_materials (profile_id, ordinal);

CREATE UNIQUE INDEX idx_workflow_profile_materials_profile_material
ON workflow_profile_materials (profile_id, material_id);

CREATE UNIQUE INDEX idx_workflow_profile_materials_ordinal
ON workflow_profile_materials (profile_id, ordinal);

CREATE TABLE workflow_profile_step_materials (
    profile_id TEXT NOT NULL,
    step_id TEXT NOT NULL,
    material_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    PRIMARY KEY (profile_id, step_id, ordinal),
    UNIQUE (profile_id, step_id, material_id),
    FOREIGN KEY (profile_id, step_id)
        REFERENCES workflow_profile_steps (profile_id, step_id) ON DELETE CASCADE,
    FOREIGN KEY (profile_id, material_id)
        REFERENCES workflow_profile_materials (profile_id, material_id) ON DELETE CASCADE
);

CREATE INDEX idx_workflow_profile_step_materials_material
ON workflow_profile_step_materials (material_id);
