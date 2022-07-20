# Transaction Parser

## Tests
- Unit tests test majority of the code.
- Integration tests using 2 files in `tests/fixtures/text*.csv`
- tests can be run as usual using `cargo test`

# Running the program
- `cargo run -- tests/fixtures/test.csv`
- `cargo run -- tests/fixtures/test2.csv`

## Approach
- We use serde and csv to parse the input file.
- serde is used to define a struct that contains the transaction values parsed from the file
- We use a HashMap to store deposit and withdrawl transactions
- After a record is parsed using the transaction.link_transaction function, we link transactions to Dispute, Resolve and Chargeback transactions.
- Since Recursive references are not allowed we use a Box type to store the linked transaction.
- We use an account struct to store the account information.
  - account has a update_account function that updates the account information based on the transaction.
  - We maintain an overall HashMap to store a map of all the accounts
- As we iterate and create a record we first parse the transaction
  - if the transaction is a withdrawal or deposit we add it to the transaction map for later lookup.
  - Each iteration we update the parsed transaction with linked transaction info (Resolve, chargeback, disputes)
  - We update the account information based on the transaction.
- When we are done we output the serialized accounts csv to stdout.

## Nuances and Assumptions
- Malformed transactions are skipped - this has been chosen over throwing an error.
- We do not handle edge cases such as negative accounts
- rust_decimal was used for easy processing of decimal types

## Safety and Efficiency
- We use box types to refer to heap values for referenced transactions. This could be a potential inefficiency.
- In memory maps are used to store transactions and accounts. These have a limitation based on the memory available.
- These in-memory maps are also only scoped for the duration of the file thus will need to leverage a global store(DB, Memcache, Redis, etc) to allow distributed processing.
- The input file is not read upfront but rather read and processed at the same time - this would allow for easy expansion to using a stream or set of streams
- The main method has been kept slim and the functions are fairly modular to allow future expansion.

## Background 
### Input
The input will be a CSV file with the columns type, client, tx, and amount. You can assume the type is a string, the client column is a valid u16 client ID, the tx is a valid u32 transaction ID, and the amount is a decimal value with a precision of up to four places past the decimal.
For example
```
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
```
- The client ID will be unique per client though are not guaranteed to be ordered. 
- Transactions to the client account 2 could occur before transactions to the client account 1. Likewise, transaction IDs (tx) are globally unique, though are also not guaranteed to be ordered. 
- You can assume the transactions occur chronologically in the file, so if transaction b appears after a in the input file then you can assume b occurred chronologically after a. 
- Whitespaces and decimal precisions (up to four places past the decimal) must be accepted by your program.
### Output
- The output should be a list of client IDs (client), available amounts (available), held amounts (held), total amounts (total), and whether the account is locked (locked). 

The total funds that are available for trading, staking, withdrawal, etc. This should be equal to the total - held amounts
held
The total funds that are held for dispute. This should be equal to total - available amounts
total
The total funds that are available or held. This should be equal to available + held
locked
Whether the account is locked. An account is locked if a charge back occurs



For example
```
client, available, held, total, locked
1, 1.5, 0.0, 1.5, false
2, 2.0, 0.0, 2.0, false
```
Spacing and displaying decimals for round values do not matter. Row ordering also does not matter. The above output will be considered the exact same as the following
```
client,available,held,total,
2,2,0,2,false
1,1.5,0,1.5,false
```




