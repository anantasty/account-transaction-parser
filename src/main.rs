use std::collections::HashMap;
use std::{env, io};

use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;

use transaction_parser::{get_boxed_transaction, Account, Transaction, TransactionType};
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
            TransactionType::Dispute(_t) => {
                transaction.transaction_type =
                    TransactionType::Dispute(get_boxed_transaction(transaction.tx, &transactions));
            }
            TransactionType::Chargeback(_t) => {
                transaction.transaction_type = TransactionType::Chargeback(get_boxed_transaction(
                    transaction.tx,
                    &transactions,
                ));
            }
            TransactionType::Resolve(_t) => {
                transaction.transaction_type =
                    TransactionType::Resolve(get_boxed_transaction(transaction.tx, &transactions));
            }
        }
        let account = accounts.entry(transaction.client).or_insert(Account {
            client: transaction.client,
            available: Decimal::new(0, 0),
            held: Decimal::new(0, 0),
            locked: false,
        });
        account.update_transaction(&transaction);
    }
    let mut writer = csv::Writer::from_writer(io::stdout());
    for account in accounts.values() {
        writer.serialize(account).unwrap();
    }
}
