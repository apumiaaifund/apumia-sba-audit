#![no_std]
#![forbid(unsafe_code)]

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TxCategory {
    Disbursement,
    Equipment,
    WorkingCapital,
    Payroll,
    RealEstate,
    OwnerDraw,
    Dividend,
    TaxDelinquency,
    Speculative,
    CashWithdrawal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LedgerEntry {
    pub category: TxCategory,
    pub amount_cents: i64,
    pub has_receipt: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AuditLimits {
    pub equipment_max_cents: i64,
    pub working_capital_max_cents: i64,
    pub payroll_max_cents: i64,
    pub real_estate_max_cents: i64,
    pub expected_ending_balance_cents: i64,
    pub noi_cents: i64,
    pub debt_service_cents: i64,
    pub min_dscr_bps: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AuditViolation {
    ConservationMismatch,
    CategoryCapExceeded,
    ProhibitedCategoryUsed,
    UnreceiptedCashWithdrawal,
    DscrCovenantBreach,
    ArithmeticOverflow,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AuditSummary {
    pub total_disbursed_cents: i64,
    pub total_spent_cents: i64,
    pub ending_balance_cents: i64,
    pub actual_dscr_bps: i64,
}

pub fn verify_sba_ledger(
    entries: &[LedgerEntry],
    limits: &AuditLimits,
) -> Result<AuditSummary, AuditViolation> {
    let mut disbursed: i64 = 0;
    let mut spent: i64 = 0;
    let mut eq_spent: i64 = 0;
    let mut wc_spent: i64 = 0;
    let mut pr_spent: i64 = 0;
    let mut re_spent: i64 = 0;

    for e in entries {
        match e.category {
            TxCategory::Disbursement => {
                if e.amount_cents < 0 {
                    return Err(AuditViolation::ConservationMismatch);
                }
                disbursed = disbursed
                    .checked_add(e.amount_cents)
                    .ok_or(AuditViolation::ArithmeticOverflow)?;
            }
            TxCategory::OwnerDraw
            | TxCategory::Dividend
            | TxCategory::TaxDelinquency
            | TxCategory::Speculative => {
                return Err(AuditViolation::ProhibitedCategoryUsed);
            }
            TxCategory::CashWithdrawal => {
                if !e.has_receipt {
                    return Err(AuditViolation::UnreceiptedCashWithdrawal);
                }
                let out = e.amount_cents.checked_neg().ok_or(AuditViolation::ArithmeticOverflow)?;
                spent = spent.checked_add(out).ok_or(AuditViolation::ArithmeticOverflow)?;
            }
            _ => {
                let out = e.amount_cents.checked_neg().ok_or(AuditViolation::ArithmeticOverflow)?;
                if out < 0 {
                    return Err(AuditViolation::ConservationMismatch);
                }
                spent = spent.checked_add(out).ok_or(AuditViolation::ArithmeticOverflow)?;
                match e.category {
                    TxCategory::Equipment => {
                        eq_spent = eq_spent.checked_add(out).ok_or(AuditViolation::ArithmeticOverflow)?;
                    }
                    TxCategory::WorkingCapital => {
                        wc_spent = wc_spent.checked_add(out).ok_or(AuditViolation::ArithmeticOverflow)?;
                    }
                    TxCategory::Payroll => {
                        pr_spent = pr_spent.checked_add(out).ok_or(AuditViolation::ArithmeticOverflow)?;
                    }
                    TxCategory::RealEstate => {
                        re_spent = re_spent.checked_add(out).ok_or(AuditViolation::ArithmeticOverflow)?;
                    }
                    _ => {}
                }
            }
        }
    }

    if eq_spent > limits.equipment_max_cents
        || wc_spent > limits.working_capital_max_cents
        || pr_spent > limits.payroll_max_cents
        || re_spent > limits.real_estate_max_cents
    {
        return Err(AuditViolation::CategoryCapExceeded);
    }

    let ending = disbursed
        .checked_sub(spent)
        .ok_or(AuditViolation::ArithmeticOverflow)?;
    if ending != limits.expected_ending_balance_cents {
        return Err(AuditViolation::ConservationMismatch);
    }

    if limits.debt_service_cents <= 0 {
        return Err(AuditViolation::DscrCovenantBreach);
    }
    let scaled_noi = limits
        .noi_cents
        .checked_mul(10_000)
        .ok_or(AuditViolation::ArithmeticOverflow)?;
    let actual_dscr_bps = scaled_noi / limits.debt_service_cents;
    if actual_dscr_bps < limits.min_dscr_bps {
        return Err(AuditViolation::DscrCovenantBreach);
    }

    Ok(AuditSummary {
        total_disbursed_cents: disbursed,
        total_spent_cents: spent,
        ending_balance_cents: ending,
        actual_dscr_bps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_limits() -> AuditLimits {
        AuditLimits {
            equipment_max_cents: 7_000_000,
            working_capital_max_cents: 5_000_000,
            payroll_max_cents: 3_000_000,
            real_estate_max_cents: 0,
            expected_ending_balance_cents: 6_000_000,
            noi_cents: 3_000_000,
            debt_service_cents: 2_000_000,
            min_dscr_bps: 12_500,
        }
    }

    #[test]
    fn test_valid_sba_audit() {
        let entries = [
            LedgerEntry { category: TxCategory::Disbursement, amount_cents: 15_000_000, has_receipt: true },
            LedgerEntry { category: TxCategory::Equipment, amount_cents: -6_500_000, has_receipt: true },
            LedgerEntry { category: TxCategory::WorkingCapital, amount_cents: -2_500_000, has_receipt: true },
        ];
        let res = verify_sba_ledger(&entries, &sample_limits()).unwrap();
        assert_eq!(res.ending_balance_cents, 6_000_000);
        assert_eq!(res.actual_dscr_bps, 15_000);
    }

    #[test]
    fn test_prohibited_owner_draw() {
        let entries = [
            LedgerEntry { category: TxCategory::Disbursement, amount_cents: 15_000_000, has_receipt: true },
            LedgerEntry { category: TxCategory::OwnerDraw, amount_cents: -9_000_000, has_receipt: true },
        ];
        assert_eq!(
            verify_sba_ledger(&entries, &sample_limits()),
            Err(AuditViolation::ProhibitedCategoryUsed)
        );
    }
}
