-- Persist the immutable policy identity and generic recommendation that produced a decision.
-- All columns remain nullable so historical records stay readable after the migration.
ALTER TABLE decision_records ADD COLUMN policy_id TEXT;
ALTER TABLE decision_records ADD COLUMN policy_version INTEGER;
ALTER TABLE decision_records ADD COLUMN recommendation_snapshot TEXT;

CREATE INDEX idx_decision_records_policy_reference
    ON decision_records(policy_id, policy_version);

CREATE TRIGGER decision_records_policy_evidence_insert_check
BEFORE INSERT ON decision_records
WHEN (NEW.policy_id IS NULL) != (NEW.policy_version IS NULL)
  OR (NEW.policy_id IS NULL) != (NEW.recommendation_snapshot IS NULL)
  OR (NEW.policy_id IS NOT NULL AND (
        length(NEW.policy_id) = 0
        OR length(NEW.policy_id) > 64
        OR NEW.policy_id NOT GLOB '[a-z]*'
        OR NEW.policy_id GLOB '*[^a-z0-9_]*'
        OR NEW.policy_version <= 0
        OR json_valid(NEW.recommendation_snapshot) = 0
        OR json_type(NEW.recommendation_snapshot) = 'null'
  ))
BEGIN
    SELECT RAISE(ABORT, 'invalid decision record policy evidence');
END;

CREATE TRIGGER decision_records_policy_evidence_update_check
BEFORE UPDATE OF policy_id, policy_version, recommendation_snapshot ON decision_records
WHEN (NEW.policy_id IS NULL) != (NEW.policy_version IS NULL)
  OR (NEW.policy_id IS NULL) != (NEW.recommendation_snapshot IS NULL)
  OR (NEW.policy_id IS NOT NULL AND (
        length(NEW.policy_id) = 0
        OR length(NEW.policy_id) > 64
        OR NEW.policy_id NOT GLOB '[a-z]*'
        OR NEW.policy_id GLOB '*[^a-z0-9_]*'
        OR NEW.policy_version <= 0
        OR json_valid(NEW.recommendation_snapshot) = 0
        OR json_type(NEW.recommendation_snapshot) = 'null'
  ))
BEGIN
    SELECT RAISE(ABORT, 'invalid decision record policy evidence');
END;
