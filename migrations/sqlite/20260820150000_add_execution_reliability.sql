-- V1.1 execution-reliability limits and actual-fill reconciliation links.
ALTER TABLE plan_execution_configurations
    ADD COLUMN opportunity_cash_cap TEXT NULL;

ALTER TABLE plan_execution_configurations
    ADD COLUMN period_execution_limit TEXT NULL;

-- SQLite cannot alter a CHECK constraint in place. Rebuild this leaf table so
-- `carry_with_cap` is representable while preserving every prior configuration.
CREATE TABLE plan_execution_configurations_next (
    plan_id TEXT PRIMARY KEY NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    schedule_kind TEXT NOT NULL,
    schedule_day INTEGER NOT NULL,
    core_ratio_units INTEGER NOT NULL,
    opportunity_ratio_units INTEGER NOT NULL,
    risk_mode TEXT NOT NULL,
    opportunity_cash_policy TEXT NOT NULL,
    schedule_days_json TEXT NOT NULL,
    opportunity_cash_cap TEXT NULL,
    period_execution_limit TEXT NULL,
    CHECK (schedule_kind IN ('monthly', 'weekly')),
    CHECK ((schedule_kind = 'monthly' AND schedule_day BETWEEN 1 AND 28) OR (schedule_kind = 'weekly' AND schedule_day BETWEEN 1 AND 7)),
    CHECK (core_ratio_units BETWEEN 0 AND 100000000),
    CHECK (opportunity_ratio_units BETWEEN 0 AND 100000000),
    CHECK (core_ratio_units + opportunity_ratio_units = 100000000),
    CHECK (risk_mode IN ('fixed', 'autopilot', 'approval')),
    CHECK ((core_ratio_units = 100000000 AND opportunity_ratio_units = 0 AND risk_mode = 'fixed') OR (core_ratio_units < 100000000 AND risk_mode IN ('autopilot', 'approval'))),
    CHECK (opportunity_cash_policy IN ('expire_each_period', 'carry_forward', 'carry_with_cap')),
    CHECK ((opportunity_cash_policy = 'carry_with_cap' AND opportunity_cash_cap IS NOT NULL) OR (opportunity_cash_policy != 'carry_with_cap' AND opportunity_cash_cap IS NULL))
);

INSERT INTO plan_execution_configurations_next
SELECT plan_id, schedule_kind, schedule_day, core_ratio_units, opportunity_ratio_units, risk_mode,
       opportunity_cash_policy, schedule_days_json, opportunity_cash_cap, period_execution_limit
FROM plan_execution_configurations;

DROP TABLE plan_execution_configurations;
ALTER TABLE plan_execution_configurations_next RENAME TO plan_execution_configurations;
CREATE INDEX plan_execution_configurations_schedule_idx
    ON plan_execution_configurations (schedule_kind, schedule_day, plan_id);

ALTER TABLE paper_orders
    ADD COLUMN decision_record_id TEXT NULL;

ALTER TABLE opportunity_cash_events
    ADD COLUMN core_contribution TEXT NOT NULL DEFAULT '000000000000.00000000';

ALTER TABLE opportunity_cash_events
    ADD COLUMN cash_cap TEXT NULL;

ALTER TABLE opportunity_cash_events
    ADD COLUMN actual_allocated_amount TEXT NULL;

ALTER TABLE opportunity_cash_events
    ADD COLUMN reconciled_at TEXT NULL;

CREATE INDEX paper_orders_decision_record_idx
    ON paper_orders (decision_record_id)
    WHERE decision_record_id IS NOT NULL;

-- This independent reservation ledger makes the per-period cap atomic before an
-- accepted order reaches the paper broker. A failed broker call releases its row.
CREATE TABLE plan_period_execution_reservations (
    decision_record_id TEXT PRIMARY KEY NOT NULL REFERENCES decision_records(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    period_key TEXT NOT NULL,
    amount TEXT NOT NULL,
    state TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (state IN ('reserved', 'accepted', 'released')),
    CHECK (period_key GLOB '????-??' OR period_key GLOB '????-W??'),
    CHECK (amount GLOB '[0-9]*.[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]')
);

CREATE INDEX plan_period_execution_reservations_plan_period_idx
    ON plan_period_execution_reservations (plan_id, period_key, state);
