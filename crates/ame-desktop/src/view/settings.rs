use nekowg::{
    Context, FontWeight, MouseButton, Render, SharedString, Subscription, Window, div, prelude::*,
    px, rgb,
};

use crate::component::{button, theme};
use crate::entity::app::CloseBehavior;
use crate::entity::runtime::AppRuntime;
use crate::entity::services::shell;

#[derive(Debug, Clone)]
pub struct SettingsViewModel {
    pub close_behavior_label: SharedString,
}

pub struct SettingsPageView {
    runtime: AppRuntime,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPageView {
    pub fn new(runtime: AppRuntime, _cx: &mut Context<Self>) -> Self {
        Self {
            runtime,
            _subscriptions: Vec::new(),
        }
    }

    fn set_close_behavior(&mut self, value: CloseBehavior, cx: &mut Context<Self>) {
        shell::set_close_behavior(&self.runtime, value, cx);
    }
}

impl Render for SettingsPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = SettingsViewModel {
            close_behavior_label: self.runtime.shell.read(cx).close_behavior.label().into(),
        };
        let page = cx.entity();
        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(32.))
            .gap_6()
            .child(
                div()
                    .text_size(px(42.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                    .child("设置"),
            )
            .child(
                div()
                    .w_full()
                    .rounded_lg()
                    .bg(rgb(theme::COLOR_CARD_DARK))
                    .px_4()
                    .py_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_color(rgb(theme::COLOR_SECONDARY))
                    .child(format!("关闭行为: {}", model.close_behavior_label))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child({
                                let page = page.clone();
                                button::pill_base("隐藏到托盘").on_mouse_down(
                                    MouseButton::Left,
                                    move |_, _, cx| {
                                        page.update(cx, |this, cx| {
                                            this.set_close_behavior(CloseBehavior::HideToTray, cx);
                                        });
                                    },
                                )
                            })
                            .child({
                                let page = page.clone();
                                button::pill_base("每次询问").on_mouse_down(
                                    MouseButton::Left,
                                    move |_, _, cx| {
                                        page.update(cx, |this, cx| {
                                            this.set_close_behavior(CloseBehavior::Ask, cx);
                                        });
                                    },
                                )
                            })
                            .child({
                                let page = page.clone();
                                button::pill_base("直接退出").on_mouse_down(
                                    MouseButton::Left,
                                    move |_, _, cx| {
                                        page.update(cx, |this, cx| {
                                            this.set_close_behavior(CloseBehavior::Exit, cx);
                                        });
                                    },
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}
