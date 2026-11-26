# E2B MicroVM Sandbox Integration (APUMIA Deterministic Core)

Run `#![no_std]` Rust and Zero-Float Python auditors inside isolated E2B Firecracker MicroVMs with egress lockdown (`deny 0.0.0.0/0`).

```bash
git clone https://github.com/apumiaaifund/apumia-sba-audit.git
cd apumia-sba-audit
cargo test --manifest-path core-rust/Cargo.toml
python3 cli-python/sba_audit.py --batch-dir ./client_portfolios
```

- Live Verifier: https://apumiaaifund.github.io/apumia-sba-audit/
- Autonomy Hub (30 Systems): https://apumiaaifund.github.io/
