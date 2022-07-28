use csv::Reader;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{Error, ErrorKind};
use std::str::FromStr;
///
/// # TransactionParser
///

/// Types of possible transactions
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Serialization for TransactionType
/// We need this to let serde play well with parsing our enums
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

/// serde + csv enum parsing code
impl<'de> Deserialize<'de> for TransactionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Parsed data - Each row results in a transaction object.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

impl Transaction {
    /// Link relevant transaction to Dispute, Chargeback or Resolve transaction
    pub fn link_transaction(&mut self, transactions: &HashMap<u32, Transaction>) {
        match &self.transaction_type {
            TransactionType::Dispute(_t) => {
                self.transaction_type =
                    TransactionType::Dispute(get_boxed_transaction(self.tx, transactions));
            }
            TransactionType::Chargeback(_t) => {
                self.transaction_type =
                    TransactionType::Chargeback(get_boxed_transaction(self.tx, transactions));
            }
            TransactionType::Resolve(_t) => {
                self.transaction_type =
                    TransactionType::Resolve(get_boxed_transaction(self.tx, transactions));
            }
            _ => {}
        }
    }

    /// Get account balance with a default value of Zero instead of None
    fn amount(&self) -> Decimal {
        match self.amount {
            Some(amount) => amount,
            None => Decimal::zero(),
        }
    }
}

/// Account to hold data of an account
#[derive(Debug, PartialEq, Eq)]
pub struct Account {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

/// Serialization for Account
impl Serialize for Account {
    // Since we need to serialize the account
    // With all fields and the total fiend which is computed
    // We cant use the #[derive(Serialize)] macro
    // We need to implement it ourself
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
    /// Return the value of held + available of the account
    pub fn total(&self) -> Decimal {
        self.available + self.held
    }

    /// Update accounts based on received transaction
    pub fn update_transaction(&mut self, transaction: &Transaction) {
        match &transaction.transaction_type {
            TransactionType::Deposit => {
                self.available += transaction.amount();
            }
            TransactionType::Withdrawal => {
                self.available -= transaction.amount();
            }
            TransactionType::Dispute(ref_transaction) => {
                match ref_transaction {
                    Some(t) => {
                        self.held += t.amount();
                        self.available -= t.amount();
                    }
                    None => {}
                }
            }
            TransactionType::Resolve(ref_transaction) => {
                match ref_transaction {
                    Some(t) => {
                        self.held -= t.amount();
                        self.available += t.amount();
                    }
                    None => {}
                }

            }
            TransactionType::Chargeback(ref_transaction) => {
                match ref_transaction {
                    Some(t) => {
                        self.held -= t.amount();
                        self.available -= t.amount();
                        self.locked = true;
                    }
                    None => {}
                }
            }
        }
    }
}

fn get_boxed_transaction(
    tx: u32,
    transactions: &HashMap<u32, Transaction>,
) -> Option<Box<Transaction>> {
    /// Convenience method to convert Option<Transaction> to Option<Box<Transaction>>
    transactions.get(&tx).map(|t| Box::new(t.clone()))
}

/// Outputs accounts to stdout
pub fn write_stdout(accounts: &HashMap<u16, Account>) {
    let mut writer = csv::Writer::from_writer(io::stdout());
    for account in accounts.values() {
        writer.serialize(account).unwrap();
    }
}

/// Accepts a reader object.
/// The function reads file line by line - creates a transaction per line
/// stores relevant value in an accounts map
pub fn process_transactions(reader: &mut Reader<File>) -> HashMap<u16, Account> {
    let mut accounts: HashMap<u16, Account> = HashMap::new();
    // maintain map or Deposit/ Withdrawal transactions
    // To use with Dispute/ Resolve/ Chargeback transactions
    let mut transactions: HashMap<u32, Transaction> = HashMap::new();
    for mut transaction in reader.deserialize::<Transaction>().flatten() {
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
    accounts
}

#[cfg(test)]
mod tests {
    use crate::{get_boxed_transaction, Account, Transaction, TransactionType};
    use rust_decimal::prelude::Zero;
    use rust_decimal::Decimal;
    use std::collections::HashMap;

    fn read_transaction(line: &str) -> Transaction {
        let mut reader = csv::Reader::from_reader(line.as_bytes());
        reader.deserialize().next().unwrap().unwrap()
    }

    #[test]
    fn parse_deposit() {
        let result = Transaction {
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let line = "type,client,tx,amount
deposit,1,1,1.0";
        let record: Transaction = read_transaction(line);
        assert_eq!(result, record);
    }

    #[test]
    fn parse_withdrawal() {
        let result = Transaction {
            transaction_type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let line = "type,client,tx,amount
withdrawal,1,1,1.0";
        let record: Transaction = read_transaction(line);
        assert_eq!(result, record);
    }

    #[test]
    fn parse_chargeback() {
        let result = Transaction {
            transaction_type: TransactionType::Chargeback(None),
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let line = "type,client,tx,amount
chargeback,1,1,1.0";
        let record: Transaction = read_transaction(line);
        assert_eq!(result, record);
    }

    #[test]
    fn parse_dispute() {
        let result = Transaction {
            transaction_type: TransactionType::Dispute(None),
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let line = "type,client,tx,amount
dispute,1,1,1.0";
        let record: Transaction = read_transaction(line);
        assert_eq!(result, record);
    }

    #[test]
    fn parse_resolve() {
        let result = Transaction {
            transaction_type: TransactionType::Resolve(None),
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let line = "type,client,tx,amount
resolve,1,1,1.0";
        let record: Transaction = read_transaction(line);
        assert_eq!(result, record);
    }

    #[test]
    fn parse_transaction_with_no_amount() {
        let result = Transaction {
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: None,
        };
        let line = "type,client,tx,amount
deposit,1,1,";
        let record: Transaction = read_transaction(line);
        assert_eq!(result, record);
    }

    #[test]
    fn deposits() {
        let mut account = Account {
            client: 1,
            available: Decimal::zero(),
            held: Decimal::zero(),
            locked: false,
        };
        let transaction = Transaction {
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        account.update_transaction(&transaction);
        assert_eq!(account.available, Decimal::new(1, 0));
        account.update_transaction(&transaction); // Add 1 again
        assert_eq!(account.available, Decimal::new(2, 0));
    }

    #[test]
    fn withdrawal() {
        let mut account = Account {
            client: 1,
            available: Decimal::new(1, 0),
            held: Decimal::zero(),
            locked: false,
        };
        let transaction = Transaction {
            transaction_type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        account.update_transaction(&transaction);
        assert_eq!(account.available, Decimal::zero());
    }

    #[test]
    fn dispute() {
        let mut account = Account {
            client: 1,
            available: Decimal::new(1, 0),
            held: Decimal::zero(),
            locked: false,
        };
        let transaction_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let transaction_dispute = Transaction {
            transaction_type: TransactionType::Dispute(Some(Box::new(transaction_deposit))),
            client: 1,
            tx: 2,
            amount: None,
        };
        account.update_transaction(&transaction_dispute);
        assert_eq!(account.available, Decimal::zero());
        assert_eq!(account.held, Decimal::new(1, 0));
    }

    #[test]
    fn resolve() {
        let mut account = Account {
            client: 1,
            available: Decimal::new(1, 0),
            held: Decimal::new(1, 0),
            locked: false,
        };
        let transaction_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let transaction_resolve = Transaction {
            transaction_type: TransactionType::Resolve(Some(Box::new(transaction_deposit))),
            client: 1,
            tx: 2,
            amount: None,
        };
        account.update_transaction(&transaction_resolve);
        assert_eq!(account.available, Decimal::new(2, 0));
        assert_eq!(account.held, Decimal::zero());
    }

    #[test]
    fn chargeback() {
        let mut account = Account {
            client: 1,
            available: Decimal::new(1, 0),
            held: Decimal::new(1, 0),
            locked: false,
        };
        let transaction_chargeback = Transaction {
            transaction_type: TransactionType::Chargeback(Some(Box::new(Transaction {
                transaction_type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(Decimal::new(1, 0)),
            }))),
            client: 1,
            tx: 2,
            amount: None,
        };
        account.update_transaction(&transaction_chargeback);
        assert_eq!(account.available, Decimal::zero());
        assert_eq!(account.held, Decimal::zero());
        assert_eq!(account.locked, true);
    }

    #[test]
    fn link_transaction() {
        let transaction_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(1, 0)),
        };
        let mut transaction_dispute = Transaction {
            transaction_type: TransactionType::Dispute(None),
            client: 1,
            tx: 1,
            amount: None,
        };
        let transaction_dispute_result = Transaction {
            transaction_type: TransactionType::Dispute(Some(Box::new(transaction_deposit.clone()))),
            client: 1,
            tx: 1,
            amount: None,
        };
        let transaction_map: HashMap<u32, Transaction> =
            HashMap::from([(1u32, transaction_deposit.clone())]);
        let boxed = get_boxed_transaction(1u32, &transaction_map);
        assert_eq!(boxed, Some(Box::new(transaction_deposit)));
        transaction_dispute.link_transaction(&transaction_map);
        assert_eq!(transaction_dispute, transaction_dispute_result);
    }
}
