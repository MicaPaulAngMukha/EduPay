# EduPay

> Instant, verifiable tuition payments on Stellar — no cashier required.

---

## Problem

STI College students and parents in Bonifacio Global City must physically visit campus to pay tuition, then return **again** to have the payment reflected on their student balance. Online payments take up to a week to process and require manual cashier verification before an official receipt is issued.

## Solution

EduPay lets parents send tuition payments instantly through Stellar. A Soroban smart contract verifies each payment, automatically updates the student's official on-chain balance, and issues a digital receipt — without any cashier involvement or follow-up visit.

---

## Stellar Features Used

| Feature | Usage |
|---|---|
| **XLM / USDC transfers** | Tuition payments denominated in XLM (or USDC via anchor) |
| **Soroban smart contracts** | On-chain balance tracking, receipt issuance, duplicate-payment guard |
| **Trustlines** | School wallet accepts tuition token |
| **Built-in DEX** *(optional)* | Forex conversion for OFW parents paying from abroad |

---

## MVP Core Feature

A Soroban smart contract that:

1. Receives tuition payments in XLM / USDC.
2. Matches each payment to a student ID and billing period.
3. Updates the student's on-chain tuition status to `PAID` or reduces the outstanding balance (`PARTIAL`).
4. Prevents duplicate payments on a fully-settled billing period.
5. Instantly generates a verifiable digital receipt linked to the Stellar ledger sequence number.

---

## Why This Wins

EduPay demonstrates Stellar's real-world utility for Southeast Asia's large private-education sector — a high-frequency, high-trust payment use case with millions of Filipino families as end-users. It showcases Soroban's ability to replace slow, manual institutional processes with auditable on-chain logic, and opens a direct channel for OFW parents to settle tuition abroad via Stellar's built-in DEX.

---

## Optional Edge — OFW Remittance Flow

Parents working abroad can pay tuition directly via Stellar's built-in DEX: send foreign currency → auto-convert to USDC → EduPay contract settles tuition in one atomic transaction. Zero forex middlemen. Receipt arrives in seconds.

---

## Timeline

| Phase | Description |
|---|---|
| **Day 1** | Contract design, data structures, `enroll` + `pay` functions |
| **Day 2** | Tests, edge cases, testnet deployment |
| **Day 3** | Frontend dashboard (parent + admin views), demo polish |

---

## Contract Structure

```
edu_pay/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs     # Soroban contract (DataKey enum, StudentRecord, Receipt, EduPay impl)
    └── test.rs    # 5 unit tests
```

### Public functions

| Function | Caller | Description |
|---|---|---|
| `initialize(admin)` | Deployer | One-time setup; stores the school admin address |
| `enroll(student_id, period, amount)` | Admin | Registers student with their tuition bill |
| `pay(payer, student_id, period, amount)` | Parent / Student | Processes payment; returns a Receipt |
| `get_record(student_id)` | Anyone | Returns StudentRecord (balance + status) |
| `get_receipts(student_id)` | Anyone | Returns all receipts for a student |
| `is_period_paid(student_id, period)` | Anyone | Returns true if period is fully settled |

---

## Prerequisites

- **Rust** ≥ 1.74 with the `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **Stellar CLI** ≥ 22.x:
  ```bash
  cargo install --locked stellar-cli
  ```
- **Testnet account** funded via Friendbot:
  ```bash
  curl "https://friendbot.stellar.org?addr=YOUR_PUBLIC_KEY"
  ```

---

## Build

```bash
stellar contract build
```

Compiled Wasm output:
```
target/wasm32-unknown-unknown/release/edu_pay.wasm
```

---

## Test

```bash
cargo test
```

Expected passing tests:

- `test_full_payment_happy_path`
- `test_duplicate_payment_rejected`
- `test_partial_payment_state`
- `test_receipt_accumulation`
- `test_unenrolled_student_rejected`

---

## Deploy to Testnet

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/edu_pay.wasm \
  --source ADMIN_SECRET_KEY \
  --network testnet
```

Copy the returned **contract ID** for all subsequent invocations.

---

## CLI Invocations

### Initialize

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- initialize \
  --admin GADMIN_PUBLIC_KEY
```

### Enroll a student

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- enroll \
  --student_id "STI-2025-00123" \
  --period "2025-1" \
  --amount 500000000000
```

*(500,000,000,000 stroops = 50,000 XLM)*

### Pay tuition — the MVP function

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source PARENT_SECRET_KEY \
  --network testnet \
  -- pay \
  --payer GPARENT_PUBLIC_KEY \
  --student_id "STI-2025-00123" \
  --period "2025-1" \
  --amount 500000000000
```

Expected response:

```json
{
  "student_id": "STI-2025-00123",
  "period": "2025-1",
  "amount_paid": 500000000000,
  "ledger": 1234567,
  "settled": true
}
```

### Check student record

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source ANY_KEY \
  --network testnet \
  -- get_record \
  --student_id "STI-2025-00123"
```

---

## License

MIT © 2025 EduPay