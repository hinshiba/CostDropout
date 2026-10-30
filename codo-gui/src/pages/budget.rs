use crate::app::AppState;
use codo_core::ledger::period::Period;
use gpui_kit::component::button::Button;
use gpui_kit::component::date_picker::{DatePicker, DatePickerState};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub struct BudgetPage {
    state: Entity<AppState>,
    amount: Entity<InputState>,
    start_date: Entity<DatePickerState>,
    end_date: Entity<DatePickerState>,
    error_message: Option<String>,
}

impl BudgetPage {
    pub fn new(
        state: Entity<AppState>,
        amount: Entity<InputState>,
        start_date: Entity<DatePickerState>,
        end_date: Entity<DatePickerState>,
    ) -> Self {
        Self {
            state,
            amount,
            start_date,
            end_date,
            error_message: None,
        }
    }

    fn add_budget(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let amount_text = self.amount.read(cx).value().to_string();
        let Ok(amount) = amount_text.parse::<i32>() else {
            self.error_message = Some("金額が不正です".to_string());
            cx.notify();
            return;
        };

        let start = self
            .start_date
            .read(cx)
            .date_time()
            .start()
            .map(|date| date.date());

        let end = self
            .end_date
            .read(cx)
            .date_time()
            .start()
            .map(|date| date.date());

        let Some(start) = start else {
            self.error_message = Some("開始日を選択してください".to_string());
            cx.notify();
            return;
        };

        let Some(end) = end else {
            self.error_message = Some("終了日を選択してください".to_string());
            cx.notify();
            return;
        };

        let Ok(period) = Period::new(start, end) else {
            self.error_message = Some("期間が正しくありません".to_string());
            cx.notify();
            return;
        };

        let result = self.state.update(cx, |state, cx| {
            state.ledger.add_budget(amount.into(), period)?;

            cx.notify();

            Ok::<_, codo_core::ledger::LedgerError>(())
        });

        match result {
            Ok(_) => {
                self.amount.update(cx, |state, cx| {
                    state.set_value("", window, cx);
                });
                self.error_message = None;
            }
            Err(error) => {
                self.error_message = Some(error.to_string());
            }
        }
        cx.notify();
    }
}

impl Render for BudgetPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let budgets = self.state.read(cx).ledger.budgets();
        //UI部分
        div().size_full().p_6().child(
            div()
                .max_w(px(560.))
                .flex()
                .flex_col()
                .gap_4()
                .child(div().text_xl().child("予算一覧"))
                //入力フォーム
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child("開始日")
                        .child(DatePicker::new(&self.start_date))
                        .child("終了日")
                        .child(DatePicker::new(&self.end_date))
                        .child("予算額")
                        .child(Input::new(&self.amount))
                        .child(
                            Button::new("add-budget")
                                .label("追加")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.add_budget(window, cx);
                                })),
                        ),
                )
                .when_some(self.error_message.clone(), |this, message| {
                    this.child(div().text_color(rgb(0xc62828)).child(message))
                })
                //一覧
                .children(budgets.iter().map(|budget| {
                    div()
                        .flex()
                        .gap_4()
                        .child(format!(
                            "{} ~ {}",
                            budget.period().start(),
                            budget.period().end(),
                        ))
                        .child(budget.amount().to_string())
                })),
        )
    }
}
