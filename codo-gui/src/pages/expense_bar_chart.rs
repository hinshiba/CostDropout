//! 品目ごとの支出推移

use crate::app::AppState;
use chrono::Local;
use codo_core::item::{ItemId, Tag};
use codo_core::money::MoneyError;
use codo_core::period::{Period, YearMonth};
use codo_core::report::trend::monthly_expense;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Selectable;
use gpui_kit::component::button::Button;
use gpui_kit::component::chart::BarChart;
use gpui_kit::*;

struct ExpenseData {
    month: String,
    amount: f64,
    is_dummy: bool,
}

pub struct ExpenseBarChart {
    state: Entity<AppState>,
    selected_item: Option<ItemId>,
}

impl ExpenseBarChart {
    pub fn new(state: Entity<AppState>) -> Self {
        Self {
            state,
            selected_item: None,
        }
    }

    fn expense_items(&self, cx: &Context<Self>) -> Vec<(ItemId, String)> {
        self.state
            .read(cx)
            .ledger
            .items()
            .iter()
            .filter(|item| item.tag() == Tag::Expense)
            .map(|item| (item.id(), item.name().to_string()))
            .collect()
    }

    fn current_period() -> Period {
        let current_month = YearMonth::from_date(Local::now().date_naive());

        let mut start_month = current_month;

        for _ in 0..11 {
            start_month = start_month.prev();
        }

        Period::new(start_month.first_day(), current_month.last_day())
            .expect("12か月の期間は常に有効なはずです")
    }

    fn chart_data(&self, cx: &Context<Self>) -> Result<Vec<ExpenseData>, MoneyError> {
        let Some(item) = self.selected_item else {
            return Ok(Vec::new());
        };

        monthly_expense(
            &self.state.read(cx).ledger,
            Self::current_period(),
            Some(item),
        )
        .map(|expenses| {
            expenses
                .into_iter()
                .map(|(month, amount)| ExpenseData {
                    month: month.to_string(),
                    amount: amount.0 as f64,
                    is_dummy: false,
                })
                .collect()
        })
    }

    fn select_item(&mut self, item_id: ItemId, cx: &mut Context<Self>) {
        self.selected_item = Some(item_id);
        cx.notify();
    }

    /// データの最大値に応じて、Y軸の上限を決める。
    fn adjusted_max(data: &[ExpenseData]) -> f64 {
        let max_amount = data.iter().map(|data| data.amount).fold(0.0, f64::max);

        if max_amount <= 50_000.0 {
            50_000.0
        } else if max_amount <= 100_000.0 {
            100_000.0
        } else if max_amount <= 200_000.0 {
            200_000.0
        } else if max_amount <= 300_000.0 {
            300_000.0
        } else if max_amount <= 400_000.0 {
            400_000.0
        } else if max_amount <= 500_000.0 {
            500_000.0
        } else if max_amount <= 600_000.0 {
            600_000.0
        } else if max_amount <= 700_000.0 {
            700_000.0
        } else if max_amount <= 800_000.0 {
            800_000.0
        } else if max_amount <= 900_000.0 {
            900_000.0
        } else if max_amount <= 1_000_000.0 {
            1_000_000.0
        } else {
            (max_amount / 500_000.0).ceil() * 500_000.0
        }
    }

    /// Y軸の目盛り数を決める。
    fn tick_count(data: &[ExpenseData]) -> usize {
        let adjusted_max = Self::adjusted_max(data);

        let step = if adjusted_max <= 100_000.0 {
            25_000.0
        } else {
            100_000.0
        };

        (adjusted_max / step) as usize + 1
    }
}

impl Render for ExpenseBarChart {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self.expense_items(cx);

        // 初回表示時は最初の支出品目を自動選択する。
        if self.selected_item.is_none()
            && let Some((item_id, _)) = items.first()
        {
            self.selected_item = Some(*item_id);
        }

        let chart_data = self.chart_data(cx);

        let item_selector = items.iter().fold(
            div().flex().flex_wrap().gap_2(),
            |container, (item_id, item_name)| {
                let selected = self.selected_item == Some(*item_id);
                let item_id = *item_id;

                container.child(
                    Button::new(format!("expense-item-{item_id:?}"))
                        .label(item_name.clone())
                        .selected(selected)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_item(item_id, cx);
                        })),
                )
            },
        );

        let chart = match chart_data {
            Ok(data) if data.is_empty() => div()
                .w_full()
                .h_96()
                .child("支出品目を選択してください")
                .into_any_element(),

            Ok(mut data) => {
                let adjusted_max = Self::adjusted_max(&data);
                let tick_count = Self::tick_count(&data);

                data.push(ExpenseData {
                    month: String::new(),
                    amount: adjusted_max,
                    is_dummy: true,
                });

                let chart_color = cx.theme().chart_1;
                let transparent = cx.theme().transparent;

                div()
                    .w_full()
                    .h_96()
                    .overflow_hidden()
                    .child(
                        div().w(relative(1.08)).h_full().child(
                            BarChart::new(data)
                                .band(|data| data.month.clone())
                                .value(|data| data.amount)
                                .label(|data| {
                                    if data.is_dummy {
                                        String::new()
                                    } else {
                                        format!("{:.0}", data.amount)
                                    }
                                })
                                .fill(move |data, _, _, _| {
                                    if data.is_dummy {
                                        transparent
                                    } else {
                                        chart_color
                                    }
                                })
                                .value_axis(true)
                                .value_tick_count(tick_count),
                        ),
                    )
                    .into_any_element()
            }

            Err(error) => div()
                .w_full()
                .h_96()
                .child(format!("集計エラー: {error:?}"))
                .into_any_element(),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_6()
            .p_6()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child("支出推移（品目別）"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_lg().child("品目"))
                    .child(item_selector),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_lg().child("過去12か月の支出"))
                    .child(chart),
            )
    }
}
