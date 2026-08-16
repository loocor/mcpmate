ALTER TABLE profile
ADD COLUMN profile_mode TEXT NOT NULL DEFAULT 'capability'
CHECK (profile_mode IN ('capability', 'workflow'));

CREATE TABLE workflow_profile_specifications (
    profile_id TEXT PRIMARY KEY,
    specification_revision INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_steps (
    profile_id TEXT NOT NULL,
    step_index INTEGER NOT NULL CHECK (step_index >= 0),
    title TEXT NOT NULL,
    description TEXT,
    PRIMARY KEY (profile_id, step_index),
    FOREIGN KEY (profile_id) REFERENCES workflow_profile_specifications (profile_id) ON DELETE CASCADE
);

CREATE TABLE workflow_profile_step_bindings (
    profile_id TEXT NOT NULL,
    step_index INTEGER NOT NULL,
    binding_index INTEGER NOT NULL CHECK (binding_index >= 0),
    ref_id TEXT NOT NULL,
    binding_policy TEXT NOT NULL DEFAULT 'meta_on_demand'
        CHECK (binding_policy IN ('meta_on_demand', 'direct')),
    expected_state_generation INTEGER NOT NULL,
    expected_capability_id TEXT NOT NULL,
    PRIMARY KEY (profile_id, step_index, binding_index),
    FOREIGN KEY (profile_id, step_index)
        REFERENCES workflow_profile_steps (profile_id, step_index) ON DELETE CASCADE,
    FOREIGN KEY (ref_id) REFERENCES capability_refs (ref_id) ON DELETE RESTRICT
);

CREATE INDEX idx_workflow_profile_step_bindings_ref
ON workflow_profile_step_bindings (ref_id);
