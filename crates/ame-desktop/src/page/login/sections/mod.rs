mod qr;

use std::rc::Rc;

use nekowg::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, rgb};

use crate::component::{button, theme};
use crate::page::login::models::LoginViewModel;

use self::qr::render_qr_area;

pub(crate) type LoginActionHandler = Rc<dyn Fn(&mut App)>;

pub(crate) fn render_login_page(
    model: LoginViewModel,
    on_generate_qr: LoginActionHandler,
    on_stop_polling: LoginActionHandler,
    on_guest_login: LoginActionHandler,
    on_refresh_login: LoginActionHandler,
) -> AnyElement {
    let qr_area = render_qr_area(model.qr_image.clone());
    let qr_url = model.qr_url.clone().unwrap_or_else(|| "无".to_string());
    let qr_status = model
        .qr_status
        .clone()
        .unwrap_or_else(|| "未开始".to_string());
    let account = model
        .account_summary
        .clone()
        .unwrap_or_else(|| "无".to_string());

    let error = model
        .error
        .as_ref()
        .map(|error| {
            div()
                .w_full()
                .rounded_lg()
                .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
                .px_4()
                .py_3()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child(error.clone())
                .into_any_element()
        })
        .unwrap_or_else(|| div().into_any_element());

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
                .child("登录"),
        )
        .child(error)
        .child(
            div()
                .w_full()
                .rounded_lg()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .px_4()
                .py_3()
                .flex()
                .flex_col()
                .gap_2()
                .child(format!("当前状态: {}", model.auth_state))
                .child(format!("账号信息: {account}"))
                .child(format!("二维码状态: {qr_status}"))
                .child(format!(
                    "轮询中: {}",
                    if model.polling { "是" } else { "否" }
                ))
                .child(div().text_color(rgb(theme::COLOR_SECONDARY)).child(qr_url)),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .gap_6()
                .child(qr_area)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child({
                            let on_generate_qr = on_generate_qr.clone();
                            button::pill_base("生成二维码")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    on_generate_qr(cx)
                                })
                        })
                        .child({
                            let on_stop_polling = on_stop_polling.clone();
                            button::pill_base("停止轮询")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    on_stop_polling(cx)
                                })
                        })
                        .child({
                            let on_guest_login = on_guest_login.clone();
                            button::primary_pill("游客登录")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    on_guest_login(cx)
                                })
                        })
                        .child(
                            button::pill_base("刷新登录态")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    on_refresh_login(cx)
                                }),
                        ),
                ),
        )
        .into_any_element()
}
