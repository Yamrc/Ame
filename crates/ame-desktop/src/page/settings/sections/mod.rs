mod option_row;

use std::rc::Rc;

use nekowg::{AnyElement, App, FontWeight, div, prelude::*, px, rgb};

use crate::component::theme;
use crate::domain::settings::{CloseBehavior, HomeArtistLanguage};
use crate::page::settings::models::SettingsViewModel;

use self::option_row::setting_option_row;

pub(crate) type CloseBehaviorHandler = Rc<dyn Fn(CloseBehavior, &mut App)>;
pub(crate) type HomeArtistLanguageHandler = Rc<dyn Fn(HomeArtistLanguage, &mut App)>;

pub(crate) fn render_settings_page(
    model: SettingsViewModel,
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
