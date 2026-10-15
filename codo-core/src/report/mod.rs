enum Flow{
    Income,
    Expense,
    Other,
}

fn flow_of(ledger: &Ledger, tx: &Transaction) -> Flow {

}

fn flows_in(ledger: &Ledger, period: Period) -> impl Iterator<Item = (&Transaction, Flow){

}

fn expense_total(ledger: &Ledger, period: Period) -> Money {

}

fn income_total(ledger: &Ledger, period: Period) -> Money {

}