#!/usr/bin/env python3
"""Build the immutable calibration-v2 fixture from committed public-source snapshots.

The script intentionally performs no network IO.  It turns the committed raw
snapshots into a dated monthly decision set. Each decision uses only
observations known at that month's final observation, then records the first
strictly later price observation as its execution price. Run it after
deliberately replacing a raw source snapshot, then review the generated diff
and update the manifest hashes.
"""

from __future__ import annotations

import csv
import hashlib
import json
from bisect import bisect_right
from collections import defaultdict
from datetime import date, datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "crates/strategy-evaluation/data/raw"
OUT = ROOT / "crates/strategy-evaluation/data/generated/calibration-v2.json"
MANIFEST = ROOT / "crates/strategy-evaluation/data/generated/calibration-v2.manifest.json"
QWEN_SENSITIVITY = ROOT / "crates/strategy-evaluation/data/generated/qwen-sensitivity-v1.json"
START = date(2005, 1, 1)
END = date(2026, 6, 30)


def parse_iso(value: str) -> date:
    return date.fromisoformat(value.strip())


def read_fred_daily(path: Path, field: str) -> list[tuple[date, float]]:
    rows = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            try:
                observed = parse_iso(row["observation_date"])
                value = float(row[field])
            except (KeyError, TypeError, ValueError):
                continue
            rows.append((observed, value))
    return rows


def read_vix(path: Path) -> list[tuple[date, float]]:
    rows = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            try:
                observed = datetime.strptime(row["DATE"], "%m/%d/%Y").date()
                value = float(row["CLOSE"])
            except (KeyError, TypeError, ValueError):
                continue
            if START <= observed <= END:
                rows.append((observed, value))
    return rows


def read_shiller(path: Path) -> dict[tuple[int, int], float]:
    values: dict[tuple[int, int], float] = {}
    text = path.read_text(encoding="utf-8")
    for row in text.split("<tr")[1:]:
        cells = [
            cell.split(">")[-1].replace("&#x2002;", " ").strip()
            for cell in row.split("</td>")[:-1]
        ]
        if len(cells) < 2:
            continue
        try:
            observed = datetime.strptime(cells[0], "%b %d, %Y").date()
            value = float(cells[1])
        except (TypeError, ValueError):
            continue
        if START <= observed <= END and value > 0:
            values[(observed.year, observed.month)] = value
    return values


def monthly_last(rows: list[tuple[date, float]]) -> dict[tuple[int, int], float]:
    output: dict[tuple[int, int], float] = {}
    for observed, value in rows:
        output[(observed.year, observed.month)] = value
    return output


def technical_monthly(rows: list[tuple[date, float]]) -> dict[tuple[int, int], dict[str, float | str]]:
    monthly: dict[tuple[int, int], dict[str, float | str]] = {}
    for index in range(199, len(rows)):
        observed, close = rows[index]
        if not START <= observed <= END:
            continue
        window = rows[index + 1 - 200 : index + 1]
        average = sum(value for _, value in window) / 200
        changes = [
            window_index[1][1] - window_index[0][1]
            for window_index in zip(rows[index + 1 - 14 : index], rows[index + 2 - 14 : index + 1])
        ]
        gains = sum(change for change in changes if change > 0)
        losses = -sum(change for change in changes if change < 0)
        rsi = 100.0 if losses == 0 else 100.0 - 100.0 / (1.0 + gains / losses)
        monthly[(observed.year, observed.month)] = {
            "decision_as_of": observed.isoformat(),
            "decision_close": round(close, 8),
            "ma200_distance": round(close / average - 1.0, 12),
            "rsi14": round(rsi, 12),
        }
    return monthly


def build_asset(
    asset_id: str,
    display_name: str,
    source_symbol: str,
    prices: list[tuple[date, float]],
    cape: dict[tuple[int, int], float],
    treasury: dict[tuple[int, int], float],
    vix: dict[tuple[int, int], float],
) -> dict[str, object]:
    observations = []
    price_dates = [observed for observed, _ in prices]
    decision_prices = [
        (observed, close) for observed, close in prices if START <= observed <= END
    ]
    for month, technical in sorted(technical_monthly(decision_prices).items()):
        cape_value = cape.get(month)
        treasury_value = treasury.get(month)
        vix_value = vix.get(month)
        if cape_value is None or treasury_value is None or vix_value is None:
            continue
        decision_date = parse_iso(str(technical["decision_as_of"]))
        execution_index = bisect_right(price_dates, decision_date)
        if execution_index == len(prices):
            continue
        execution_date, execution_close = prices[execution_index]
        observations.append(
            {
                **technical,
                "execution_as_of": execution_date.isoformat(),
                "execution_close": round(execution_close, 8),
                "cape": round(cape_value, 8),
                "erp_proxy": round(100.0 / cape_value - treasury_value, 12),
                "vix": round(vix_value, 8),
            }
        )
    return {
        "id": asset_id,
        "display_name": display_name,
        "source_symbol": source_symbol,
        "observations": observations,
    }


def main() -> None:
    cape = read_shiller(RAW / "multpl_shiller_pe.html")
    treasury = monthly_last(read_fred_daily(RAW / "fred_dgs10_daily.csv", "DGS10"))
    vix = monthly_last(read_vix(RAW / "cboe_vix_daily.csv"))
    assets = [
        build_asset(
            "sp500_index_proxy",
            "S&P 500 index (SPY proxy)",
            "SP500",
            read_fred_daily(RAW / "fred_sp500_daily.csv", "SP500"),
            cape,
            treasury,
            vix,
        ),
        build_asset(
            "nasdaq_composite_proxy",
            "NASDAQ Composite index (QQQ proxy)",
            "NASDAQCOM",
            read_fred_daily(RAW / "fred_nasdaqcom_daily.csv", "NASDAQCOM"),
            cape,
            treasury,
            vix,
        ),
    ]
    payload = {
        "schema_version": 2,
        "dataset_version": "calibration-v2",
        "capture_date": "2026-08-21",
        "range": {"start": START.isoformat(), "end": END.isoformat()},
        "frequency": "monthly final available decision observation; execute at first strictly later daily price observation",
        "assets": assets,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    sources = [
        (
            "fred_sp500_daily.csv",
            "https://fred.stlouisfed.org/graph/fredgraph.csv?id=SP500",
            "FRED SP500 daily index; used as an SPY proxy because the committed public snapshot does not claim ETF execution prices.",
        ),
        (
            "fred_nasdaqcom_daily.csv",
            "https://fred.stlouisfed.org/graph/fredgraph.csv?id=NASDAQCOM",
            "FRED NASDAQ Composite daily index; used as a QQQ proxy, not as QQQ execution prices.",
        ),
        (
            "fred_dgs10_daily.csv",
            "https://fred.stlouisfed.org/graph/fredgraph.csv?id=DGS10",
            "Daily ten-year Treasury yield; the last valid observation in a calendar month is used.",
        ),
        (
            "cboe_vix_daily.csv",
            "https://cdn.cboe.com/api/global/us_indices/daily_prices/VIX_History.csv",
            "Daily CBOE VIX close; the last valid observation in a calendar month is used.",
        ),
        (
            "multpl_shiller_pe.html",
            "https://www.multpl.com/shiller-pe/table/by-month",
            "Monthly Shiller CAPE HTML snapshot; values are parsed by date and never interpolated.",
        ),
    ]
    manifest = {
        "dataset_version": "calibration-v2",
        "parent_dataset": "calibration-v1",
        "captured_on": "2026-08-21",
        "range": payload["range"],
        "frequency": payload["frequency"],
        "missing_value_rule": "Drop a month for an asset when any required CAPE, DGS10, VIX, technical observation, or strictly later execution price is absent; do not forward-fill or interpolate.",
        "execution_timing_rule": "Calculate after the decision observation at t and execute at the first committed daily price observation strictly after t. Never execute at the decision close.",
        "sources": [
            {
                "file": filename,
                "url": url,
                "note": note,
                "sha256": hashlib.sha256((RAW / filename).read_bytes()).hexdigest(),
            }
            for filename, url, note in sources
        ],
        "generated_fixture": {
            "file": OUT.name,
            "sha256": hashlib.sha256(OUT.read_bytes()).hexdigest(),
        },
        "frozen_qwen_sensitivity_fixture": {
            "file": QWEN_SENSITIVITY.name,
            "sha256": hashlib.sha256(QWEN_SENSITIVITY.read_bytes()).hexdigest(),
            "scope": "Distribution sensitivity only; excluded from historical performance claims.",
        },
        "generator": "tools/generate_calibration_fixture.py",
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
