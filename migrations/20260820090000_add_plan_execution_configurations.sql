-- V1.1 execution configuration for PostgreSQL investment plans.
-- Existing monthly plans remain valid and are backfilled as fixed core DCA.

CREATE TABLE investment_plan_execution_configurations (
    plan_id UUID PRIMARY KEY REFERENCES investment_plans(id) ON DELETE CASCADE,
    schedule_kind TEXT NOT NULL,
    schedule_day SMALLINT NOT NULL,
    core_ratio_units INTEGER NOT NULL,
    opportunity_ratio_units INTEGER NOT NULL,
    risk_mode TEXT NOT NULL,

    CONSTRAINT investment_plan_execution_configurations_schedule_kind_check
        CHECK (schedule_kind IN ('monthly', 'weekly')),
    CONSTRAINT investment_plan_execution_configurations_schedule_day_check
        CHECK (
            (schedule_kind = 'monthly' AND schedule_day BETWEEN 1 AND 28)
            OR (schedule_kind = 'weekly' AND schedule_day BETWEEN 1 AND 7)
        ),
    CONSTRAINT investment_plan_execution_configurations_core_ratio_check
        CHECK (core_ratio_units BETWEEN 0 AND 100000000),
    CONSTRAINT investment_plan_execution_configurations_opportunity_ratio_check
        CHECK (opportunity_ratio_units BETWEEN 0 AND 100000000),
    CONSTRAINT investment_plan_execution_configurations_ratio_sum_check
        CHECK (core_ratio_units + opportunity_ratio_units = 100000000),
    CONSTRAINT investment_plan_execution_configurations_risk_mode_check
        CHECK (risk_mode IN ('fixed', 'autopilot', 'approval')),
    CONSTRAINT investment_plan_execution_configurations_risk_mode_matches_buckets_check
        CHECK (
            (core_ratio_units = 100000000 AND opportunity_ratio_units = 0 AND risk_mode = 'fixed')
            OR (core_ratio_units < 100000000 AND risk_mode IN ('autopilot', 'approval'))
        )
);

INSERT INTO investment_plan_execution_configurations (
    plan_id,
    schedule_kind,
    schedule_day,
    core_ratio_units,
    opportunity_ratio_units,
    risk_mode
)
SELECT id, 'monthly', schedule_day, 100000000, 0, 'fixed'
FROM investment_plans;

CREATE INDEX investment_plan_execution_configurations_schedule_idx
    ON investment_plan_execution_configurations (schedule_kind, schedule_day, plan_id);
