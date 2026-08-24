-- Bind every persisted plan to an immutable policy version.
-- Existing plans were created under the legacy 70/20/10 behaviour and must retain it.
ALTER TABLE plan_execution_configurations
    ADD COLUMN policy_id TEXT NOT NULL DEFAULT 'core_opportunity_v1';

ALTER TABLE plan_execution_configurations
    ADD COLUMN policy_version INTEGER NOT NULL DEFAULT 1 CHECK (policy_version > 0);

CREATE INDEX idx_plan_execution_configurations_policy
    ON plan_execution_configurations (policy_id, policy_version);
