#![feature(box_patterns)]

use std::io::{Error, ErrorKind};
use std::str::FromStr;
use std::collections::HashMap;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub enum TransactionType {
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
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
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
pub struct Account {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
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
    pub fn total(&self) -> Decimal {
        self.available + self.held
    }

    // Update accounts based on received transaction
    pub fn update_transaction(&mut self, transaction: &Transaction) {
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

pub fn get_boxed_transaction(
    tx: u32,
    transactions: &HashMap<u32, Transaction>,
) -> Option<Box<Transaction>> {
    transactions.get(&tx).map(|t| Box::new(t.clone()))
}