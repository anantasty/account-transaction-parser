use std::env;

use transaction_parser::{process_transactions, write_stdout};

fn main() {
    // Since we are only accepting the first positional argument
    // There is no need for a more advanced parser like clap
    let args: Vec<String> = env::args().collect();
    let mut reader = csv::Reader::from_path(args[1].clone()).unwrap();
    let accounts = process_transactions(&mut reader);
    write_stdout(&accounts);
}
