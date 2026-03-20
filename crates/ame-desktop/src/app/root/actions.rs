use nekowg::Context;
use tracing::warn;

use crate::app::route::AppRoute;
use crate::app::router;
use crate::domain::player;
use crate::domain::settings::CloseBehavior;
use crate::domain::shell as shell_domain;

use super::RootView;

impl RootView {
    pub(crate) fn navigate_to(&mut self, route: AppRoute, cx: &mut Context<Self>) {
        router::navigate_route(cx, route);
        cx.notify();
    }

    pub(crate) fn request_window_close(
        &mut self,
        window: &mut nekowg::Window,
        cx: &mut Context<Self>,
    ) {
        match self.env.shell().read(cx).close_behavior {
            CloseBehavior::HideToTray => {
                window.hide();
            }
            CloseBehavior::Exit => {
                self.prepare_app_exit(cx);
                cx.quit();
            }
            CloseBehavior::Ask => {
                let window_handle = window.window_handle();
                let answer = window.prompt(
                    nekowg::PromptLevel::Info,
                    "确定要关闭吗？",
                    Some("以下选择会作为默认行为，可以在设置中修改"),
                    &[
                        nekowg::PromptButton::new("隐藏到托盘"),
                        nekowg::PromptButton::ok("退出应用"),
                        nekowg::PromptButton::cancel("取消"),
                    ],
                    cx,
                );
                let root = cx.entity();
                cx.spawn(async move |_, cx| {
                    let choice = match answer.await {
                        Ok(choice) => choice,
                        Err(err) => {
                            warn!("window close prompt failed: {err}");
                            return;
                        }
                    };
                    root.update(cx, |this, cx| match choice {
                        0 => {
                            shell_domain::set_close_behavior(
                                &this.runtime,
                                CloseBehavior::HideToTray,
                                cx,
                            );
                            if let Err(err) = window_handle.update(cx, |_, window, _cx| {
                                window.hide();
                            }) {
                                warn!("window hide after close prompt failed: {err}");
                            }
                        }
                        1 => {
                            shell_domain::set_close_behavior(
                                &this.runtime,
                                CloseBehavior::Exit,
                                cx,
                            );
                            this.prepare_app_exit(cx);
                            cx.quit();
                        }
                        _ => {}
                    });
                })
                .detach();
            }
        }
    }

    pub(crate) fn prepare_app_exit(&mut self, cx: &mut Context<Self>) {
        player::prepare_app_exit(&self.runtime, cx);
    }

    pub(crate) fn tray_toggle_playback(&mut self, cx: &mut Context<Self>) {
        player::toggle_playback(&self.runtime, cx);
    }

    pub(crate) fn tray_next(&mut self, cx: &mut Context<Self>) {
        player::play_next(&self.runtime, cx);
    }

    pub(crate) fn tray_previous(&mut self, cx: &mut Context<Self>) {
        player::play_previous(&self.runtime, cx);
    }
}
