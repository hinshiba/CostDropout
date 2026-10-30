//! 支出推移

use crate::app::AppState;
use chrono::{Datelike, Local};
use codo_core::item::{ItemId, Tag};
use codo_core::money::MoneyError;
use codo_core::period::{Period, YearMonth};
use codo_core::report::trend::{monthly_expense, yearly_expense};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Selectable;
use gpui_kit::component::button::Button;
use gpui_kit::component::chart::BarChart;
use gpui_kit::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChartMode {
    Item,
    Total,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TotalMode {
    Yearly,
    Monthly,
}

struct ExpenseData {
    label: String,
    amount: f64,
    is_dummy: bool,
}

pub struct ExpenseBarChart {
    state: Entity<AppState>,
    chart_mode: ChartMode,
    total_mode: TotalMode,
    selected_item: Option<ItemId>,
    selected_year: i32,
}

impl ExpenseBarChart {
    pub fn new(state: Entity<AppState>) -> Self {
        Self {
            state,
            chart_mode: ChartMode::Item,
            total_mode: TotalMode::Yearly,
            selected_item: None,
            selected_year: Local::now().year(),
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
        let ledger = &self.state.read(cx).ledger;

        // 表示モードに応じて、グラフに使用する支出データを切り替える。
        match self.chart_mode {
            ChartMode::Item => {
                let Some(item) = self.selected_item else {
                    return Ok(Vec::new());
                };

                monthly_expense(ledger, Self::current_period(), Some(item)).map(|expenses| {
                    expenses
                        .into_iter()
                        .map(|(month, amount)| ExpenseData {
                            label: month.to_string(),
                            amount: f64::from(amount),
                            is_dummy: false,
                        })
                        .collect()
                })
            }

            ChartMode::Total => match self.total_mode {
                TotalMode::Yearly => yearly_expense(ledger).map(|expenses| {
                    expenses
                        .into_iter()
                        .map(|(year, amount)| ExpenseData {
                            label: format!("{year}年"),
                            amount: f64::from(amount),
                            is_dummy: false,
                        })
                        .collect()
                }),

                TotalMode::Monthly => {
                    monthly_expense(ledger, Period::year(self.selected_year), None).map(
                        |expenses| {
                            expenses
                                .into_iter()
                                .map(|(month, amount)| ExpenseData {
                                    label: format!("{}月", month.month()),
                                    amount: f64::from(amount),
                                    is_dummy: false,
                                })
                                .collect()
                        },
                    )
                }
            },
        }
    }

    fn select_chart_mode(&mut self, mode: ChartMode, cx: &mut Context<Self>) {
        self.chart_mode = mode;
        cx.notify();
    }

    fn select_total_mode(&mut self, mode: TotalMode, cx: &mut Context<Self>) {
        self.total_mode = mode;
        cx.notify();
    }

    fn select_item(&mut self, item_id: ItemId, cx: &mut Context<Self>) {
        self.selected_item = Some(item_id);
        cx.notify();
    }

    fn previous_year(&mut self, cx: &mut Context<Self>) {
        self.selected_year -= 1;
        cx.notify();
    }

    fn next_year(&mut self, cx: &mut Context<Self>) {
        self.selected_year += 1;
        cx.notify();
    }

    /// データの最大値に応じて、Y軸の上限を決める。
    ///
    /// max_amount が 100,000 以下の場合は 50,000 刻み、
    /// 1,000,000 以下の場合は 100,000 刻み、
    /// 1,000,000 より大きい場合は 500,000 刻みで、
    /// 最大値以上となる上限を設定する。
    fn adjusted_max(data: &[ExpenseData]) -> f64 {
        let max_amount = data.iter().map(|data| data.amount).fold(0.0, f64::max);

        let step = if max_amount <= 100_000.0 {
            50_000.0
        } else if max_amount <= 1_000_000.0 {
            100_000.0
        } else {
            500_000.0
        };

        (max_amount / step).ceil().max(1.0) * step
    }

    /// Y軸の目盛り数を決める。
    fn tick_count(data: &[ExpenseData]) -> usize {
        let adjusted_max = Self::adjusted_max(data);

        let step = if adjusted_max <= 100_000.0 {
            25_000.0
        } else {
            100_000.0
        };

        ((adjusted_max / step) as usize + 1).min(11)
    }
}

impl Render for ExpenseBarChart {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self.expense_items(cx);

        // 選択中の品目が現在も支出品目に存在するか確認する。
        // 存在しなければ、先頭の支出品目を選択する。
        // 支出品目がなければ None にする。
        if !self
            .selected_item
            .is_some_and(|selected| items.iter().any(|(item_id, _)| *item_id == selected))
        {
            self.selected_item = items.first().map(|(item_id, _)| *item_id);
        }

        let chart_data = self.chart_data(cx);

        let chart_mode_selector = div()
            .flex()
            .gap_2()
            .child(
                Button::new("chart-mode-item")
                    .label("品目別")
                    .selected(self.chart_mode == ChartMode::Item)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_chart_mode(ChartMode::Item, cx);
                    })),
            )
            .child(
                Button::new("chart-mode-total")
                    .label("合計")
                    .selected(self.chart_mode == ChartMode::Total)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_chart_mode(ChartMode::Total, cx);
                    })),
            );

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

        let total_mode_selector = div()
            .flex()
            .gap_2()
            .child(
                Button::new("total-mode-yearly")
                    .label("年別")
                    .selected(self.total_mode == TotalMode::Yearly)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_total_mode(TotalMode::Yearly, cx);
                    })),
            )
            .child(
                Button::new("total-mode-monthly")
                    .label("月別")
                    .selected(self.total_mode == TotalMode::Monthly)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_total_mode(TotalMode::Monthly, cx);
                    })),
            );

        let year_selector = if self.total_mode == TotalMode::Monthly {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Button::new("previous-year")
                        .label("←")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.previous_year(cx);
                        })),
                )
                .child(div().text_lg().child(format!("{}年", self.selected_year)))
                .child(Button::new("next-year").label("→").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.next_year(cx);
                    },
                )))
        } else {
            div()
        };

        let chart = match chart_data {
            Ok(data) if data.is_empty() => div()
                .w_full()
                .h_96()
                .child("支出データがありません")
                .into_any_element(),

            Ok(mut data) => {
                let adjusted_max = Self::adjusted_max(&data);
                let tick_count = Self::tick_count(&data);

                data.push(ExpenseData {
                    label: String::new(),
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
                        div().w(relative(1.12)).h_full().child(
                            BarChart::new(data)
                                .band(|data| data.label.clone())
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
                .child(format!("集計エラー: {error}"))
                .into_any_element(),
        };

        let controls = match self.chart_mode {
            ChartMode::Item => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_lg().child("品目"))
                .child(item_selector)
                .into_any_element(),

            ChartMode::Total => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(total_mode_selector)
                .child(year_selector)
                .into_any_element(),
        };

        let chart_title = match self.chart_mode {
            ChartMode::Item => "過去12か月の支出",
            ChartMode::Total => match self.total_mode {
                TotalMode::Yearly => "年ごとの支出",
                TotalMode::Monthly => "月ごとの支出",
            },
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
                    .child("支出推移"),
            )
            .child(chart_mode_selector)
            .child(controls)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_lg().child(chart_title))
                    .child(chart),
            )
    }
}
