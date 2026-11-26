import json
from pathlib import Path

import pytest
from sba_audit import verify_sba_files


def test_compliant_ledger(tmp_path: Path) -> None:
    ledger = tmp_path / "ledger.csv"
    ledger.write_text(
        "tx_id,date,category,amount_cents,counterparty,receipt_hash\n"
        "TX-1,2026-11-01,DISBURSEMENT,15000000,SBA,sha256:111\n"
        "TX-2,2026-11-02,EQUIPMENT,-6500000,VENDOR,sha256:222\n"
        "TX-3,2026-11-03,WORKING_CAPITAL,-2500000,OPS,sha256:333\n",
        encoding="utf-8",
    )
    limits = tmp_path / "limits.json"
    limits.write_text(
        json.dumps({
            "equipment_max_cents": 7000000,
            "working_capital_max_cents": 5000000,
            "payroll_max_cents": 3000000,
            "real_estate_max_cents": 0,
            "expected_ending_balance_cents": 6000000,
            "noi_cents": 3000000,
            "debt_service_cents": 2000000,
            "min_dscr_bps": 12500,
        }),
        encoding="utf-8",
    )
    res = verify_sba_files(ledger, limits)
    assert res["status"] == "VERIFIED_COMPLIANT"
    assert res["actual_dscr_bps"] == 15000
    assert len(res["canonical_sha256"]) == 64


def test_owner_draw_rejected(tmp_path: Path) -> None:
    ledger = tmp_path / "ledger.csv"
    ledger.write_text(
        "tx_id,date,category,amount_cents,counterparty,receipt_hash\n"
        "TX-1,2026-11-01,OWNER_DRAW,-100000,FOUNDER,\n",
        encoding="utf-8",
    )
    limits = tmp_path / "limits.json"
    limits.write_text('{"expected_ending_balance_cents":0,"noi_cents":1,"debt_service_cents":1,"min_dscr_bps":1}', encoding="utf-8")
    with pytest.raises(ValueError, match="Prohibited category"):
        verify_sba_files(ledger, limits)
