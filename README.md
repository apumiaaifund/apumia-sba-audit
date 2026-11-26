# apumia-sba-audit

Deterministic Zero-Float (`#![no_std]` Rust + Python `int`/cents) Use-of-Proceeds & DSCR Covenant Auditor for US SBA 7(a)/504 borrowers and CPAs.

- **Live Air-Gapped Verifier:** https://apumiaaifund.github.io/apumia-sba-audit/
- **APUMIA Autonomy Hub (29 Systems):** https://apumiaaifund.github.io/

## Invariants Verified
1. `INV-01`: Exact cent conservation (`Disbursed - Spent - EndingBalance == 0`).
2. `INV-02`: Category cap enforcement (`Equipment`, `WorkingCapital`, `Payroll`, `RealEstate`).
3. `INV-03`: Hard block on prohibited categories (`OwnerDraw`, `Dividend`, `TaxDelinquency`, `Speculative`).
4. `INV-04`: Receipt hash enforcement on cash withdrawals.
5. `INV-05`: Integer basis-point DSCR covenant verification (`NOI * 10000 // DebtService >= min_dscr_bps`).
