#![feature(box_patterns)]

use std::collections::HashMap;
use std::{env, io};
use std::io::{Error, ErrorKind};

use std::str::FromStr;

use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeStruct;

#[derive(Debug, Clone)]
enum TransactionType {
    Deposit,
    Withdrawal,
    // Box type to avoid type Recursion
    // Storing referenced transaction
    // on Heap is a better solution than
    // having to pass a reference to transactions
    // Every time we update an account
    Dispute(Option<Box<Transaction>>),
    Resolve(Option<Box<Transaction>>),
    Chargeback(Option<Box<Transaction>>),
}

impl FromStr for TransactionType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deposit" => Ok(TransactionType::Deposit),
            "withdrawal" => Ok(TransactionType::Withdrawal),
            // Since we only have access to a String
            // We will add the value of the referred transaction later
            "dispute" => Ok(TransactionType::Dispute(None)),
            "resolve" => Ok(TransactionType::Resolve(None)),
            "chargeback" => Ok(TransactionType::Chargeback(None)),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "Invalid transaction type",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for TransactionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Deserialize, Clone)]
struct Transaction {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}

impl Transaction {
    fn amount(&self) -> Decimal {
        match self.amount {
            Some(amount) => amount,
            None => Decimal::zero(),
        }
    }
}

#[derive(Debug)]
struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    locked: bool,
}

// Since we need to serialize the account
// With all fields and the total fiend which is computed
// We cant use the #[derive(Serialize)] macro
// We need to implement it ourself
impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Account", 5)?;
        state.serialize_field("client", &self.client)?;
        state.serialize_field("available", &self.available)?;
        state.serialize_field("held", &self.held)?;
        state.serialize_field("locked", &self.locked)?;
        state.serialize_field("balance", &self.total())?;
        state.end()
    }
}

impl Account {
    fn total(&self) -> Decimal {
        self.available + self.held
    }

    // Update accounts based on received transaction
    fn update_transaction(&mut self, transaction: &Transaction) {
        match &transaction.transaction_type {
            TransactionType::Deposit => {
                self.available += transaction.amount();
            }
            TransactionType::Withdrawal => {
                self.available -= transaction.amount();
            }
            TransactionType::Dispute(ref_transaction) => match ref_transaction {
                Some(box t) => {
                    self.held += t.amount();
                    self.available -= t.amount();
                }
                _ => (),
            },
            TransactionType::Resolve(ref_transaction) => match ref_transaction {
                Some(box t) => {
                    self.held -= t.amount();
                    self.available += t.amount();
                }
                _ => (),
            },
            TransactionType::Chargeback(ref_transaction) => match ref_transaction {
                Some(box t) => {
                    self.held -= t.amount();
                    self.available -= t.amount();
                    self.locked = true;
                }
                _ => (),
            },
        }
    }
}

fn get_boxed_transaction(
    tx: u32,
    transactions: &HashMap<u32, Transaction>,
) -> Option<Box<Transaction>> {
    transactions.get(&tx).map(|t| Box::new(t.clone()))
}
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
