use rust_decimal::Decimal;
use transaction_parser::process_transactions;

#[test]
fn processes_file1() {
    let mut reader = csv::Reader::from_path("./tests/fixtures/test.csv").unwrap();
    let accounts = process_transactions(&mut reader);
    assert_eq!(accounts.len(), 4);
    assert_eq!(accounts.get(&2u16).unwrap().total(), Decimal::new(-1,0));
    assert_eq!(accounts.get(&1u16).unwrap().total(), Decimal::new(15,1));
    assert_eq!(accounts.get(&3u16).unwrap().total(), Decimal::new(15,1));
    assert_eq!(accounts.get(&4u16).unwrap().total(), Decimal::new(4,0));
}

#[test]
fn processes_file2() {
    let mut reader = csv::Reader::from_path("./tests/fixtures/test2.csv").unwrap();
    let accounts = process_transactions(&mut reader);
    assert_eq!(accounts.len(), 4);
    assert_eq!(accounts.get(&2u16).unwrap().total(), Decimal::new(-5,0));
    assert_eq!(accounts.get(&2u16).unwrap().locked, true);
    assert_eq!(accounts.get(&1u16).unwrap().total(), Decimal::new(15,1));
    assert_eq!(accounts.get(&3u16).unwrap().total(), Decimal::new(15,1));
    assert_eq!(accounts.get(&4u16).unwrap().total(), Decimal::new(4,0));
}

