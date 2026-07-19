-- Local-only paper-trading performance ledger.
--
-- OpenD simulated accounts do not expose a complete fill or cash-flow history.
-- These tables therefore record IndexLink-observed order state, derived fill
-- deltas and user-confirmed local cash flows. Amounts use a fixed-width decimal
-- representation so SQLite text ordering preserves numeric ordering.

CREATE TABLE cash_flows (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    amount TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT cash_flows_kind_check
        CHECK (kind IN ('opening_balance', 'deposit', 'withdrawal')),
    CONSTRAINT cash_flows_amount_check
        CHECK (amount GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT cash_flows_occurred_at_utc_check
        CHECK (strftime('%Y-%m-%dT%H:%M:%fZ', occurred_at) = occurred_at),
    CONSTRAINT cash_flows_created_at_utc_check
        CHECK (strftime('%Y-%m-%dT%H:%M:%fZ', created_at) = created_at)
);

CREATE UNIQUE INDEX cash_flows_one_opening_balance_per_plan_idx
    ON cash_flows (plan_id) WHERE kind = 'opening_balance';
CREATE INDEX cash_flows_plan_occurred_idx
    ON cash_flows (plan_id, occurred_at ASC, id ASC);

CREATE TABLE paper_orders (
    order_id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    requested_quantity TEXT NOT NULL,
    state TEXT NOT NULL,
    filled_quantity TEXT NOT NULL,
    average_fill_price TEXT NOT NULL,
    submitted_at TEXT NOT NULL,
    observed_at TEXT NOT NULL,

    CONSTRAINT paper_orders_symbol_check
        CHECK (symbol = trim(symbol) AND symbol = upper(symbol) AND length(symbol) BETWEEN 1 AND 32),
    CONSTRAINT paper_orders_side_check CHECK (side IN ('buy', 'sell')),
    CONSTRAINT paper_orders_state_check
        CHECK (state IN ('pending', 'partially_filled', 'filled', 'closed', 'unknown')),
    CONSTRAINT paper_orders_quantity_check
        CHECK (requested_quantity GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT paper_orders_filled_quantity_check
        CHECK (filled_quantity GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT paper_orders_average_fill_price_check
        CHECK (average_fill_price GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT paper_orders_submitted_at_utc_check
        CHECK (strftime('%Y-%m-%dT%H:%M:%fZ', submitted_at) = submitted_at),
    CONSTRAINT paper_orders_observed_at_utc_check
        CHECK (strftime('%Y-%m-%dT%H:%M:%fZ', observed_at) = observed_at)
);

CREATE INDEX paper_orders_plan_submitted_idx
    ON paper_orders (plan_id, submitted_at ASC, order_id ASC);

CREATE TABLE paper_fills (
    id TEXT PRIMARY KEY NOT NULL,
    order_id TEXT NOT NULL REFERENCES paper_orders(order_id) ON DELETE CASCADE,
    quantity TEXT NOT NULL,
    price TEXT NOT NULL,
    observed_at TEXT NOT NULL,

    CONSTRAINT paper_fills_quantity_check
        CHECK (quantity GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]' AND quantity > '000000000000.00000000'),
    CONSTRAINT paper_fills_price_check
        CHECK (price GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]' AND price > '000000000000.00000000'),
    CONSTRAINT paper_fills_observed_at_utc_check
        CHECK (strftime('%Y-%m-%dT%H:%M:%fZ', observed_at) = observed_at)
);

CREATE INDEX paper_fills_order_observed_idx
    ON paper_fills (order_id, observed_at ASC, id ASC);

CREATE TABLE portfolio_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    currency TEXT NOT NULL,
    symbol_price TEXT NOT NULL,
    tracked_quantity TEXT NOT NULL,
    adaptive_value TEXT NOT NULL,
    plain_dca_value TEXT NOT NULL,
    net_contributions TEXT NOT NULL,
    realized_pnl TEXT NOT NULL,
    observed_at TEXT NOT NULL,

    CONSTRAINT portfolio_snapshots_currency_check CHECK (currency GLOB '[A-Z][A-Z][A-Z]'),
    CONSTRAINT portfolio_snapshots_symbol_price_check
        CHECK (symbol_price GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT portfolio_snapshots_tracked_quantity_check
        CHECK (tracked_quantity GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT portfolio_snapshots_adaptive_value_check
        CHECK (adaptive_value GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT portfolio_snapshots_plain_dca_value_check
        CHECK (plain_dca_value GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT portfolio_snapshots_net_contributions_check
        CHECK (net_contributions GLOB '-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]' OR net_contributions GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT portfolio_snapshots_realized_pnl_check
        CHECK (realized_pnl GLOB '-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]' OR realized_pnl GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'),
    CONSTRAINT portfolio_snapshots_observed_at_utc_check
        CHECK (strftime('%Y-%m-%dT%H:%M:%fZ', observed_at) = observed_at)
);

CREATE INDEX portfolio_snapshots_plan_observed_idx
    ON portfolio_snapshots (plan_id, observed_at ASC, id ASC);
