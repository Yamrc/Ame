mod option_row;

use std::rc::Rc;

use nekowg::{AnyElement, App, FontWeight, MouseButton, div, img, prelude::*, px, rgb};

use crate::component::{button, theme};
use crate::domain::settings::{CloseBehavior, HomeArtistLanguage};
use crate::page::settings::models::{
    LastFmAccountViewModel, NeteaseAccountViewModel, SettingsViewModel,
};

use self::option_row::setting_option_row;

pub(crate) type SettingsActionHandler = Rc<dyn Fn(&mut App)>;
pub(crate) type CloseBehaviorHandler = Rc<dyn Fn(CloseBehavior, &mut App)>;
pub(crate) type HomeArtistLanguageHandler = Rc<dyn Fn(HomeArtistLanguage, &mut App)>;

pub(crate) fn render_settings_page(
    model: SettingsViewModel,
    on_generate_qr: SettingsActionHandler,
    on_refresh_login: SettingsActionHandler,
    on_connect_lastfm: SettingsActionHandler,
    on_disconnect_lastfm: SettingsActionHandler,
    on_set_close_behavior: CloseBehaviorHandler,
    on_set_home_artist_language: HomeArtistLanguageHandler,
) -> AnyElement {
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
        .child(render_app_error(model.app_error))
        .child(account_card_netease(
            model.netease,
            on_generate_qr,
            on_refresh_login,
        ))
        .child(account_card_lastfm(
            model.lastfm,
            on_connect_lastfm,
            on_disconnect_lastfm,
        ))
        .child(setting_option_row(
            format!("关闭行为: {}", model.close_behavior_label),
            [
                CloseBehavior::HideToTray,
                CloseBehavior::Ask,
                CloseBehavior::Exit,
            ]
            .into_iter()
            .map(|behavior| {
                let label = behavior.label();
                let on_set_close_behavior = on_set_close_behavior.clone();
                (
                    label,
                    Rc::new(move |cx: &mut App| on_set_close_behavior(behavior, cx))
                        as Rc<dyn Fn(&mut App)>,
                )
            })
            .collect(),
        ))
        .child(setting_option_row(
            format!("首页推荐艺人语种: {}", model.home_artist_language_label),
            HomeArtistLanguage::variants()
                .into_iter()
                .map(|language| {
                    let label = language.label();
                    let on_set_home_artist_language = on_set_home_artist_language.clone();
                    (
                        label,
                        Rc::new(move |cx: &mut App| on_set_home_artist_language(language, cx))
                            as Rc<dyn Fn(&mut App)>,
                    )
                })
                .collect(),
        ))
        .into_any_element()
}

fn render_app_error(error: Option<nekowg::SharedString>) -> AnyElement {
    match error.map(|value| value.to_string()) {
        Some(error) if !error.trim().is_empty() => div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(error)
            .into_any_element(),
        _ => div().into_any_element(),
    }
}

fn account_card_netease(
    model: NeteaseAccountViewModel,
    on_generate_qr: SettingsActionHandler,
    on_refresh_login: SettingsActionHandler,
) -> AnyElement {
    let NeteaseAccountViewModel {
        auth_state,
        account_summary,
        qr_status,
        qr_url,
        qr_image,
        polling,
    } = model;
    let qr_area = match qr_image {
        Some(qr_image) => div()
            .w(px(280.))
            .h(px(280.))
            .rounded_lg()
            .bg(rgb(theme::COLOR_BODY_BG_DARK))
            .p_2()
            .child(img(qr_image).w_full().h_full().rounded_lg())
            .into_any_element(),
        None => div()
            .w(px(280.))
            .h(px(280.))
            .rounded_lg()
            .bg(rgb(theme::COLOR_BODY_BG_DARK))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("尚未生成二维码")
            .into_any_element(),
    };

    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_4()
        .flex()
        .flex_col()
        .gap_4()
        .child(card_title("网易云账号"))
        .child(
            div()
                .w_full()
                .flex()
                .justify_between()
                .gap_6()
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .child(format!("当前状态: {}", auth_state))
                        .child(format!("账号信息: {}", account_summary))
                        .child(format!("二维码状态: {}", qr_status))
                        .child(format!("轮询中: {}", if polling { "是" } else { "否" }))
                        .child(
                            div()
                                .text_color(rgb(theme::COLOR_TEXT_DARK))
                                .child(qr_url.unwrap_or_else(|| "无".into())),
                        )
                        .child(
                            div()
                                .mt_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    button::primary_pill("生成二维码")
                                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                            on_generate_qr(cx)
                                        }),
                                )
                                .child(
                                    button::pill_base("刷新登录态")
                                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                            on_refresh_login(cx)
                                        }),
                                ),
                        ),
                )
                .child(qr_area),
        )
        .into_any_element()
}

fn account_card_lastfm(
    model: LastFmAccountViewModel,
    on_connect_lastfm: SettingsActionHandler,
    on_disconnect_lastfm: SettingsActionHandler,
) -> AnyElement {
    let action = if model.connected {
        button::pill_base("断开 Last.fm")
            .on_mouse_down(MouseButton::Left, move |_, _, cx| on_disconnect_lastfm(cx))
    } else {
        let label = if model.auth_loading {
            "连接中..."
        } else {
            "连接 Last.fm"
        };
        button::primary_pill(label)
            .on_mouse_down(MouseButton::Left, move |_, _, cx| on_connect_lastfm(cx))
    };

    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_4()
        .flex()
        .flex_col()
        .gap_4()
        .child(card_title("Last.fm"))
        .child(
            div()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .flex()
                .flex_col()
                .gap_2()
                .child(format!("当前状态: {}", model.status))
                .child(format!("账号信息: {}", model.account_summary))
                .child(model.queue_summary)
                .child(if model.configured {
                    "构建配置: 已启用".to_string()
                } else {
                    "构建配置: 未注入 API key / secret".to_string()
                }),
        )
        .child(
            model
                .error
                .map(|error| {
                    div()
                        .rounded_lg()
                        .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
                        .px_4()
                        .py_3()
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .child(error)
                        .into_any_element()
                })
                .unwrap_or_else(|| div().into_any_element()),
        )
        .child(div().flex().items_center().gap_2().child(action))
        .into_any_element()
}

fn card_title(text: impl Into<String>) -> AnyElement {
    div()
        .text_size(px(26.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(theme::COLOR_TEXT_DARK))
        .child(text.into())
        .into_any_element()
}
