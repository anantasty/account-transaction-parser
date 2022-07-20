use std::collections::HashMap;
use std::env;

use rust_decimal::Decimal;

use transaction_parser::{write_stdout, Account, Transaction, TransactionType};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut reader = csv::Reader::from_path(args[1].clone()).unwrap();
    let mut accounts: HashMap<u16, Account> = HashMap::new();
    // maintain map or Deposit/ Withdrawal transactions
    // To use with Dispute/ Resolve/ Chargeback transactions
    let mut transactions: HashMap<u32, Transaction> = HashMap::new();
    for record in reader.deserialize() {
        let mut transaction: Transaction = record.unwrap();
        match transaction.transaction_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                transactions.insert(transaction.tx, transaction.clone());
            }
            // Since we were not able to read linked transaction during parsing
            // We link them using our Map of transactions
            TransactionType::Dispute(ref _t)
            | TransactionType::Chargeback(ref _t)
            | TransactionType::Resolve(ref _t) => {
                transaction.link_transaction(&transactions);
            }
        }

        // Get an account or Create a new account with 0 balance
        // Then Update it
        let account = accounts.entry(transaction.client).or_insert(Account {
            client: transaction.client,
            available: Decimal::new(0, 0),
            held: Decimal::new(0, 0),
            locked: false,
        });
        account.update_transaction(&transaction);
    }
    write_stdout(&accounts);
}
