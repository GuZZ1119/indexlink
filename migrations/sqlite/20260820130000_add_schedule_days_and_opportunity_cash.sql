-- V1.1 executable periodic schedule and local opportunity-cash state.
-- `schedule_day` remains the first value for legacy API/database compatibility.
ALTER TABLE plan_execution_configurations
    ADD COLUMN schedule_days_json TEXT NOT NULL DEFAULT '[]';

UPDATE plan_execution_configurations
SET schedule_days_json = json_array(schedule_day)
WHERE schedule_days_json = '[]';

CREATE TABLE opportunity_cash_balances (
    plan_id TEXT PRIMARY KEY NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    balance TEXT NOT NULL DEFAULT '000000000000.00000000',
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (balance GLOB '[0-9]*.[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CHECK (updated_at GLOB '????-??-??T??:??:??.???Z')
);

CREATE TABLE opportunity_cash_events (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    decision_record_id TEXT NOT NULL REFERENCES decision_records(id) ON DELETE CASCADE,
    scheduled_for TEXT NOT NULL,
    policy TEXT NOT NULL,
    balance_before TEXT NOT NULL,
    period_budget TEXT NOT NULL,
    allocated_amount TEXT NOT NULL,
    balance_after TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (decision_record_id),
    CHECK (policy IN ('expire_each_period', 'carry_forward')),
    CHECK (scheduled_for GLOB '????-??-??'),
    CHECK (balance_before GLOB '[0-9]*.[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CHECK (period_budget GLOB '[0-9]*.[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CHECK (allocated_amount GLOB '[0-9]*.[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CHECK (balance_after GLOB '[0-9]*.[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]')
);

CREATE INDEX opportunity_cash_events_plan_scheduled_idx
    ON opportunity_cash_events (plan_id, scheduled_for);
