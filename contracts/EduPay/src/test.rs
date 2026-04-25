#[cfg(test)]
mod tests {
    use crate::{EduPay, EduPayClient};
    use soroban_sdk::{
        testutils::Address as _,
        Address, Env, String, symbol_short,
    };

    // ── Helper: spin up a fresh env, deploy, and initialise ──────────────

    fn setup() -> (Env, EduPayClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, EduPay);
        let client = EduPayClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        (env, client, admin)
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 1 – Happy path: full payment end-to-end
    //
    // Enrol a student → pay the exact billed amount → assert:
    //   • receipt is returned with settled = true
    //   • on-chain record shows outstanding = 0 and status = PAID
    //   • is_period_paid returns true
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn test_full_payment_happy_path() {
        let (env, client, _admin) = setup();

        let sid    = String::from_str(&env, "STI-2025-00123");
        let period = String::from_str(&env, "2025-1");
        let tuition: i128 = 50_000 * 10_000_000; // 50,000 XLM in stroops

        client.enroll(&sid, &period, &tuition);

        let payer   = Address::generate(&env);
        let receipt = client.pay(&payer, &sid, &period, &tuition);

        // Receipt fields.
        assert_eq!(receipt.student_id,  sid);
        assert_eq!(receipt.period,      period);
        assert_eq!(receipt.amount_paid, tuition);
        assert!(receipt.settled, "receipt.settled must be true after full payment");

        // On-chain record.
        let record = client.get_record(&sid);
        assert_eq!(record.outstanding, 0);
        assert_eq!(record.status, symbol_short!("PAID"));

        // Period flag.
        assert!(
            client.is_period_paid(&sid, &period),
            "period must be flagged as paid in storage"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 2 – Edge case: duplicate payment rejected
    //
    // After a period is fully settled, a second pay() call must panic with
    // the duplicate-payment message.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    #[should_panic(expected = "tuition for this period is already fully paid")]
    fn test_duplicate_payment_rejected() {
        let (env, client, _admin) = setup();

        let sid    = String::from_str(&env, "STI-2025-00456");
        let period = String::from_str(&env, "2025-1");
        let tuition: i128 = 30_000 * 10_000_000;

        client.enroll(&sid, &period, &tuition);

        let payer = Address::generate(&env);
        client.pay(&payer, &sid, &period, &tuition); // settles the period
        client.pay(&payer, &sid, &period, &tuition); // must panic
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 3 – State verification: partial payment
    //
    // After paying half the balance:
    //   • outstanding == total − partial
    //   • status == PARTIAL
    //   • settled == false
    //   • is_period_paid == false
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn test_partial_payment_state() {
        let (env, client, _admin) = setup();

        let sid     = String::from_str(&env, "STI-2025-00789");
        let period  = String::from_str(&env, "2025-2");
        let tuition: i128 = 60_000 * 10_000_000;
        let partial: i128 = 25_000 * 10_000_000;

        client.enroll(&sid, &period, &tuition);

        let payer   = Address::generate(&env);
        let receipt = client.pay(&payer, &sid, &period, &partial);

        assert!(!receipt.settled, "receipt.settled must be false after partial payment");
        assert_eq!(receipt.amount_paid, partial);

        let record = client.get_record(&sid);
        assert_eq!(record.outstanding, tuition - partial);
        assert_eq!(record.status, symbol_short!("PARTIAL"));

        assert!(
            !client.is_period_paid(&sid, &period),
            "period must NOT be flagged as paid after partial payment"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 4 – Receipt accumulation across multiple payments
    //
    // Two payments on the same period should produce two receipts stored
    // in order; the final receipt must be marked settled.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn test_receipt_accumulation() {
        let (env, client, _admin) = setup();

        let sid     = String::from_str(&env, "STI-2025-00321");
        let period  = String::from_str(&env, "2025-1");
        let tuition: i128 = 40_000 * 10_000_000;
        let first:   i128 = 15_000 * 10_000_000;
        let second:  i128 = tuition - first; // settles balance

        client.enroll(&sid, &period, &tuition);

        let payer = Address::generate(&env);
        client.pay(&payer, &sid, &period, &first);
        client.pay(&payer, &sid, &period, &second);

        let receipts = client.get_receipts(&sid);
        assert_eq!(receipts.len(), 2, "exactly two receipts must be stored");
        assert_eq!(receipts.get(0).unwrap().amount_paid, first);
        assert_eq!(receipts.get(1).unwrap().amount_paid, second);
        assert!(
            receipts.get(1).unwrap().settled,
            "second receipt must be marked settled"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 5 – Unenrolled student rejected
    //
    // Calling pay() for a student ID that was never enrolled must panic.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    #[should_panic(expected = "student not enrolled")]
    fn test_unenrolled_student_rejected() {
        let (env, client, _admin) = setup();

        let sid    = String::from_str(&env, "STI-GHOST-00000");
        let period = String::from_str(&env, "2025-1");
        let payer  = Address::generate(&env);

        client.pay(&payer, &sid, &period, &1_000_000);
    }
}