CREATE TABLE strategy_specs (
    policy_id TEXT NOT NULL,
    policy_version INTEGER NOT NULL CHECK (policy_version > 0),
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    spec_json TEXT NOT NULL CHECK (json_valid(spec_json) AND json_type(spec_json) = 'object'),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (policy_id, policy_version)
);

CREATE INDEX idx_strategy_specs_created_at
    ON strategy_specs (created_at DESC, policy_id ASC, policy_version DESC);
