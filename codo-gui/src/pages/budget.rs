use gpui_kit::*;

pub struct BudgetPage {}

impl BudgetPage {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct BudgetRow {
    period: String,
    amount: String,
}
//仮実装なので期間と金額は固定
impl Render for BudgetPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let budgets = vec![
            BudgetRow {
                period: "2026-10-01 ~ 2026-10-31".to_string(),
                amount: "50000円".to_string(),
            },
            BudgetRow {
                period: "2026-11-01 ~ 2026-11-30".to_string(),
                amount: "60000円".to_string(),
            },
        ];
        //UI部分
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_6()
            .child(div().text_xl().child("予算一覧"))
            .children(budgets.into_iter().map(|budget| {
                div()
                    .flex()
                    .gap_4()
                    .child(budget.period)
                    .child(budget.amount)
            }))
    }
}
