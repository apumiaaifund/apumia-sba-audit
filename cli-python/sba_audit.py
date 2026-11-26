import csv
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

PROHIBITED = {"OWNER_DRAW", "DIVIDEND", "TAX_DELINQUENCY", "SPECULATIVE"}
ALLOWED_CAPS = {"EQUIPMENT", "WORKING_CAPITAL", "PAYROLL", "REAL_ESTATE"}


def _check_entry(
    row: dict[str, str],
    spent_by_cat: dict[str, int],
) -> tuple[int, int]:
    cat = row["category"].strip().upper()
    amt = int(row["amount_cents"])
    has_rcpt = row.get("receipt_hash", "").strip().startswith("sha256:")

    if cat in PROHIBITED:
        raise ValueError(f"Prohibited category used: {cat}")
    if cat == "DISBURSEMENT":
        if amt < 0:
            raise ValueError("Disbursement cannot be negative")
        return amt, 0
    if cat == "CASH_WITHDRAWAL" and not has_rcpt:
        raise ValueError("Unreceipted cash withdrawal")

    out = -amt
    if out < 0:
        raise ValueError(f"Expense must be negative: {cat}")
    if cat in ALLOWED_CAPS:
        spent_by_cat[cat] = spent_by_cat.get(cat, 0) + out
    return 0, out


def verify_sba_files(ledger_path: Path, limits_path: Path) -> dict[str, Any]:
    limits: dict[str, int] = json.loads(limits_path.read_text(encoding="utf-8"))
    disbursed = 0
    spent = 0
    spent_by_cat: dict[str, int] = {}

    with ledger_path.open("r", encoding="utf-8", newline="") as f:
        for row in csv.DictReader(f):
            d_add, s_add = _check_entry(row, spent_by_cat)
            disbursed += d_add
            spent += s_add

    for cat in ALLOWED_CAPS:
        cap_key = f"{cat.lower()}_max_cents"
        if spent_by_cat.get(cat, 0) > limits.get(cap_key, 0):
            raise ValueError(f"Category cap exceeded: {cat}")

    ending = disbursed - spent
    if ending != limits["expected_ending_balance_cents"]:
        raise ValueError("Conservation of funds mismatch")

    ds = limits["debt_service_cents"]
    if ds <= 0:
        raise ValueError("Invalid debt service")
    actual_dscr_bps = (limits["noi_cents"] * 10000) // ds
    if actual_dscr_bps < limits["min_dscr_bps"]:
        raise ValueError("DSCR covenant breach")

    payload = {
        "status": "VERIFIED_COMPLIANT",
        "total_disbursed_cents": disbursed,
        "total_spent_cents": spent,
        "ending_balance_cents": ending,
        "actual_dscr_bps": actual_dscr_bps,
    }
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    payload["canonical_sha256"] = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    return payload




def verify_batch_dir(batch_dir: Path) -> dict[str, Any]:
    results: list[dict[str, Any]] = []
    passed = 0
    failed = 0
    for d in sorted(p for p in batch_dir.iterdir() if p.is_dir()):
        ledger, limits = d.joinpath('bank_ledger.csv'), d.joinpath('limits.json')
        if not ledger.exists() or not limits.exists():
            continue
        try:
            v = verify_sba_files(ledger, limits)
            v['client'] = d.name
            results.append(v)
            passed += 1
        except ValueError as exc:
            results.append({'client': d.name, 'status': 'AUDIT_EXCEPTION', 'reason': str(exc)})
            failed += 1
    out: dict[str, Any] = {'mode': 'CPA_PORTFOLIO_BATCH', 'clients_audited': passed + failed, 'compliant_count': passed, 'exception_count': failed, 'clients': results}
    canon = json.dumps(out, sort_keys=True, separators=(',', ':'))
    out['portfolio_sha256'] = hashlib.sha256(canon.encode('utf-8')).hexdigest()
    return out


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("Usage: python3 sba_audit.py <bank_ledger.csv> <limits.json>")
    verdict = verify_sba_files(Path(sys.argv[1]), Path(sys.argv[2]))
    print(json.dumps(verdict, indent=2))
