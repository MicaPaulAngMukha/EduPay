#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, Env, String, Symbol, Vec,
    symbol_short,
};

// ─── Storage key types ─────────────────────────────────────────────────────
//
// Soroban's instance storage is a flat key→value map. We use an enum as the
// key so each logical record occupies its own slot. This avoids nested Maps,
// which require non-trivial serialization support and are fragile in practice.

#[contracttype]
pub enum DataKey {
    /// Marks the contract as initialised; value = admin Address.
    Admin,
    /// StudentRecord keyed by student ID string.
    Student(String),
    /// Vec<Receipt> keyed by student ID string.
    Receipts(String),
    /// bool keyed by (student ID, period) — true means fully settled.
    PaidPeriod(String, String),
}

// ─── Data structures ───────────────────────────────────────────────────────

/// On-chain record of a student's tuition balance for the current enrolment.
#[contracttype]
#[derive(Clone)]
pub struct StudentRecord {
    /// Total amount billed in stroops (1 XLM = 10_000_000 stroops).
    pub total_billed: i128,
    /// Remaining amount still owed, in stroops.
    pub outstanding: i128,
    /// Human-readable status: "UNPAID", "PARTIAL", or "PAID".
    pub status: Symbol,
    /// The billing period this record belongs to, e.g. "2025-1".
    pub period: String,
}

/// Digital receipt issued after each payment call.
#[contracttype]
#[derive(Clone)]
pub struct Receipt {
    /// Student ID this receipt belongs to.
    pub student_id: String,
    /// Billing period, e.g. "2025-1".
    pub period: String,
    /// Amount paid in this transaction, in stroops.
    pub amount_paid: i128,
    /// Stellar ledger sequence number — acts as an on-chain timestamp.
    pub ledger: u32,
    /// True when this payment fully settles the outstanding balance.
    pub settled: bool,
}

// ─── Contract ──────────────────────────────────────────────────────────────

#[contract]
pub struct EduPay;

#[contractimpl]
impl EduPay {

    // ── initialize ────────────────────────────────────────────────────────
    // Must be called once immediately after deployment.
    // Stores the school's admin address and guards against re-initialisation.

    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("contract already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    // ── enroll ────────────────────────────────────────────────────────────
    // Admin-only: registers a student with their tuition amount and billing
    // period. Overwrites any previous record for the same student_id so the
    // school can update bills before payment begins.

    pub fn enroll(
        env: Env,
        student_id: String,
        period: String,
        amount: i128,
    ) {
        // Verify caller is the admin.
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if amount <= 0 {
            panic!("tuition amount must be positive");
        }

        let record = StudentRecord {
            total_billed: amount,
            outstanding: amount,
            status: symbol_short!("UNPAID"),
            period,
        };

        env.storage().instance().set(&DataKey::Student(student_id), &record);
    }

    // ── pay ───────────────────────────────────────────────────────────────
    // Core MVP function. Called by the parent or student to submit a payment.
    //
    // Steps performed atomically:
    //   1. Require payer authorisation.
    //   2. Reject zero/negative amounts.
    //   3. Block duplicate payments on a fully-settled period.
    //   4. Load student record (panic if not enrolled).
    //   5. Reduce outstanding balance (floor at zero).
    //   6. Update status: PAID if settled, PARTIAL otherwise.
    //   7. Persist updated record.
    //   8. Mark period as paid in flat storage when fully settled.
    //   9. Build, persist, and return a digital receipt.

    pub fn pay(
        env: Env,
        payer: Address,
        student_id: String,
        period: String,
        amount: i128,
    ) -> Receipt {
        // Step 1 – payer must sign.
        payer.require_auth();

        // Step 2 – reject non-positive amounts.
        if amount <= 0 {
            panic!("payment amount must be positive");
        }

        // Step 3 – duplicate-payment guard.
        let period_key = DataKey::PaidPeriod(student_id.clone(), period.clone());
        if env.storage().instance().get::<DataKey, bool>(&period_key).unwrap_or(false) {
            panic!("tuition for this period is already fully paid");
        }

        // Step 4 – load student record.
        let student_key = DataKey::Student(student_id.clone());
        let mut record: StudentRecord = env
            .storage()
            .instance()
            .get(&student_key)
            .unwrap_or_else(|| panic!("student not enrolled"));

        // Step 5 – apply payment, floored at zero.
        let new_outstanding = record.outstanding.saturating_sub(amount);
        let settled = new_outstanding == 0;

        // Step 6 – update status.
        record.outstanding = new_outstanding;
        record.status = if settled {
            symbol_short!("PAID")
        } else {
            symbol_short!("PARTIAL")
        };

        // Step 7 – persist updated record.
        env.storage().instance().set(&student_key, &record);

        // Step 8 – mark period as settled in flat storage.
        if settled {
            env.storage().instance().set(&period_key, &true);
        }

        // Step 9 – build and persist digital receipt.
        let receipt = Receipt {
            student_id: student_id.clone(),
            period: period.clone(),
            amount_paid: amount,
            ledger: env.ledger().sequence(),
            settled,
        };

        let receipts_key = DataKey::Receipts(student_id);
        let mut receipts: Vec<Receipt> = env
            .storage()
            .instance()
            .get(&receipts_key)
            .unwrap_or_else(|| Vec::new(&env));

        receipts.push_back(receipt.clone());
        env.storage().instance().set(&receipts_key, &receipts);

        receipt
    }

    // ── get_record ────────────────────────────────────────────────────────
    // Returns the full StudentRecord for a given student ID.
    // Panics if the student has not been enrolled.

    pub fn get_record(env: Env, student_id: String) -> StudentRecord {
        env.storage()
            .instance()
            .get(&DataKey::Student(student_id))
            .unwrap_or_else(|| panic!("student not enrolled"))
    }

    // ── get_receipts ──────────────────────────────────────────────────────
    // Returns every receipt issued to a student, in chronological order.
    // Returns an empty Vec if the student has no receipts yet.

    pub fn get_receipts(env: Env, student_id: String) -> Vec<Receipt> {
        env.storage()
            .instance()
            .get(&DataKey::Receipts(student_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ── is_period_paid ────────────────────────────────────────────────────
    // Returns true if the given student's billing period is fully settled.

    pub fn is_period_paid(env: Env, student_id: String, period: String) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::PaidPeriod(student_id, period))
            .unwrap_or(false)
    }
}
#[cfg(test)]
mod test;