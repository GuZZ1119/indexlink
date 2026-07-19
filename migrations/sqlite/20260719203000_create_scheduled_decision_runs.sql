-- Idempotency ledger for the minimal fixed-monthly scheduler.
-- A plan can claim at most one automatic decision run for a UTC calendar day.
CREATE TABLE IF NOT EXISTS scheduled_decision_runs (
    plan_id TEXT NOT NULL,
    scheduled_for TEXT NOT NULL,
    claimed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (plan_id, scheduled_for),
    FOREIGN KEY (plan_id) REFERENCES investment_plans(id) ON DELETE CASCADE,
    CHECK (scheduled_for GLOB '????-??-??')
);
