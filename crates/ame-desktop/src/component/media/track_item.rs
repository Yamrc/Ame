use std::rc::Rc;
use std::time::Duration;

use nekowg::{
    AnyElement, App, FontWeight, HighlightStyle, MouseButton, ObjectFit, SharedString, StyledText,
    div, img, prelude::*, px, rgb, rgba,
};

use crate::animation::{Linear, TransitionExt};
use crate::component::context_menu::ContextMenuExt;
use crate::component::{button, icon, theme};
use crate::util::url::image_resize_url;

type TrackAction = Rc<dyn Fn(&mut App)>;

const ROW_COVER_SIZE: f32 = 40.;
const ROW_HEIGHT: f32 = 52.;
const ROW_HORIZONTAL_PADDING: f32 = 8.;
const ROW_VERTICAL_PADDING: f32 = 4.;
const ROW_CONTENT_GAP: f32 = 10.;
const TITLE_COLUMN_MAX_WIDTH: f32 = 420.;
const ALBUM_COLUMN_WIDTH: f32 = 240.;
const FAVORITE_BUTTON_SIZE: f32 = 28.;
const DURATION_COLUMN_WIDTH: f32 = 56.;
const META_COLUMN_WIDTH: f32 = FAVORITE_BUTTON_SIZE + 8. + DURATION_COLUMN_WIDTH;
const FAVORITE_ICON_SIZE: f32 = 15.;
const TITLE_ALIAS_COLOR: u32 = 0x6F6F6F;
const ROW_HOVER_DURATION_MS: u64 = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackItemFavoriteState {
    pub liked: bool,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackItemProps {
    pub id: i64,
    pub state_id: SharedString,
    pub title: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
    pub show_cover: bool,
    pub is_playing: bool,
    pub favorite: TrackItemFavoriteState,
}

#[derive(Clone, Default)]
pub struct TrackItemActions {
    pub on_play: Option<TrackAction>,
    pub on_enqueue: Option<TrackAction>,
    pub on_toggle_favorite: Option<TrackAction>,
    pub on_remove: Option<TrackAction>,
    pub on_open_artist: Option<TrackAction>,
    pub on_open_album: Option<TrackAction>,
}

pub fn render(props: TrackItemProps, actions: TrackItemActions) -> AnyElement {
    let title_color = if props.is_playing {
        theme::COLOR_PRIMARY
    } else {
        theme::COLOR_TEXT_DARK
    };
    let secondary_color = if props.is_playing {
        theme::COLOR_PRIMARY
    } else {
        theme::COLOR_SECONDARY
    };
    let info_color = if props.is_playing {
        theme::COLOR_PRIMARY
    } else {
        0xD8D8D8
    };
    let display_alias = props
        .alias
        .as_deref()
        .map(str::trim)
        .filter(|alias| !alias.is_empty() && *alias != props.title.trim())
        .map(str::to_string);
    let title_text = build_title_text(&props.title, display_alias.as_deref(), props.is_playing);
    let album = props.album.clone().unwrap_or_default().trim().to_string();
    let duration = props.duration_ms.map(format_duration).unwrap_or_default();
    let on_play_row = actions.on_play.clone();
    let on_toggle_favorite = actions.on_toggle_favorite.clone();
    let row_id: SharedString = format!("track-item-row-{}", props.state_id).into();
    let base_bg = if props.is_playing {
        rgba(theme::with_alpha(theme::COLOR_PRIMARY, 0x18))
    } else {
        rgba(theme::with_alpha(theme::COLOR_BODY_BG_DARK, 0x00))
    };
    let hover_bg = if props.is_playing {
        rgba(theme::with_alpha(theme::COLOR_PRIMARY, 0x22))
    } else {
        rgba(theme::with_alpha(
            theme::COLOR_SECONDARY_BG_TRANSPARENT_DARK,
            theme::ALPHA_SECONDARY_BG_TRANSPARENT,
        ))
    };

    let cover = if props.show_cover {
        Some(match props.cover_url.as_deref() {
            Some(url) => img(image_resize_url(url, "64y64"))
                .id(format!("song.cover.{}", props.state_id))
                .size(px(ROW_COVER_SIZE))
                .rounded_md()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => div()
                .size(px(ROW_COVER_SIZE))
                .rounded_md()
                .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
                .into_any_element(),
        })
    } else {
        None
    };

    let favorite_interactive = props.favorite.enabled && !props.favorite.pending;
    let favorite_icon = if props.favorite.liked {
        icon::IconName::HeartSolid
    } else {
        icon::IconName::Heart
    };
    let favorite_color = if props.favorite.liked {
        theme::COLOR_PRIMARY
    } else {
        theme::COLOR_SECONDARY
    };
    let favorite_button_id = format!("track-item-favorite-{}", props.state_id);
    let favorite_button = button::icon_interactive(
        favorite_button_id,
        button::icon_base(button::ButtonStyle::default())
            .size(px(FAVORITE_BUTTON_SIZE))
            .text_color(rgb(favorite_color))
            .when(!favorite_interactive, |this| {
                this.opacity(0.45).cursor_default()
            })
            .when(favorite_interactive, |this| {
                let on_toggle_favorite = on_toggle_favorite.clone();
                this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    if let Some(on_toggle_favorite) = on_toggle_favorite.as_ref() {
                        on_toggle_favorite(cx);
                    }
                })
            })
            .child(icon::render(
                favorite_icon,
                FAVORITE_ICON_SIZE,
                favorite_color,
            )),
        button::ButtonStyle::default(),
    );

    let row = div()
        .id(row_id.clone())
        .w_full()
        .min_h(px(ROW_HEIGHT))
        .rounded_lg()
        .bg(base_bg)
        .px(px(ROW_HORIZONTAL_PADDING))
        .py(px(ROW_VERTICAL_PADDING))
        .flex()
        .items_center()
        .gap(px(ROW_CONTENT_GAP))
        .cursor_pointer()
        .when(on_play_row.is_some(), |this| {
            this.on_mouse_down(MouseButton::Left, move |event, _, cx| {
                if event.click_count >= 2
                    && let Some(on_play) = on_play_row.as_ref()
                {
                    on_play(cx);
                }
            })
        })
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .max_w(px(TITLE_COLUMN_MAX_WIDTH))
                .flex()
                .items_center()
                .gap(px(ROW_CONTENT_GAP))
                .children(cover)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .max_w_full()
                                .truncate()
                                .text_size(px(16.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(title_color))
                                .child(title_text),
                        )
                        .child(
                            div()
                                .w_full()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(rgb(secondary_color))
                                .child(props.artists.clone()),
                        ),
                ),
        )
        .child(
            div()
                .w(px(ALBUM_COLUMN_WIDTH))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_start()
                .truncate()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(info_color))
                .child(album),
        )
        .child(div().flex_1())
        .child(
            div()
                .w(px(META_COLUMN_WIDTH))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(favorite_button)
                .child(
                    div()
                        .w(px(DURATION_COLUMN_WIDTH))
                        .text_right()
                        .text_size(px(13.))
                        .text_color(rgb(info_color))
                        .child(duration),
                ),
        )
        .with_transition(row_id)
        .transition_on_hover(
            Duration::from_millis(ROW_HOVER_DURATION_MS),
            Linear,
            move |hovered, this| {
                if *hovered {
                    this.bg(hover_bg)
                } else {
                    this.bg(base_bg)
                }
            },
        );

    let menu_id = format!("track-item-menu-{}", props.state_id);
    row.context_menu_with_id(menu_id, move |menu, _window, _cx| {
        let mut menu = menu.track_header(
            props.cover_url.clone(),
            props.title.clone(),
            props.artists.clone(),
        );
        if let Some(on_play) = actions.on_play.clone() {
            menu = menu.item("播放", move |_window, cx| on_play(cx));
        }
        if let Some(on_enqueue) = actions.on_enqueue.clone() {
            menu = menu.item("入队", move |_window, cx| on_enqueue(cx));
        }
        if props.favorite.pending {
            let label = if props.favorite.liked {
                "取消收藏中..."
            } else {
                "收藏中..."
            };
            menu = menu.item_disabled(label, true, |_window, _cx| {});
        } else if props.favorite.enabled {
            if let Some(on_toggle_favorite) = actions.on_toggle_favorite.clone() {
                let label = if props.favorite.liked {
                    "取消收藏"
                } else {
                    "收藏"
                };
                menu = menu.item(label, move |_window, cx| on_toggle_favorite(cx));
            } else {
                menu = menu.item_disabled("收藏", true, |_window, _cx| {});
            }
        } else {
            let label = if props.favorite.liked {
                "取消收藏"
            } else {
                "收藏"
            };
            menu = menu.item_disabled(label, true, |_window, _cx| {});
        }
        if let Some(on_remove) = actions.on_remove.clone() {
            menu = menu.item("移出队列", move |_window, cx| on_remove(cx));
        }
        if actions.on_open_artist.is_some() || actions.on_open_album.is_some() {
            menu = menu.separator();
        }
        if let Some(on_open_artist) = actions.on_open_artist.clone() {
            menu = menu.item("打开歌手", move |_window, cx| on_open_artist(cx));
        }
        if let Some(on_open_album) = actions.on_open_album.clone() {
            menu = menu.item("打开专辑", move |_window, cx| on_open_album(cx));
        }
        menu
    })
    .into_any_element()
}

fn build_title_text(title: &str, alias: Option<&str>, is_playing: bool) -> StyledText {
    let Some(alias) = alias else {
        return StyledText::new(title.to_string());
    };

    let combined = format!("{title} ({alias})");
    let highlight = HighlightStyle {
        color: Some(
            rgb(if is_playing {
                theme::COLOR_PRIMARY
            } else {
                TITLE_ALIAS_COLOR
            })
            .into(),
        ),
        ..Default::default()
    };

    StyledText::new(combined.clone()).with_highlights([(title.len()..combined.len(), highlight)])
}

fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes}:{seconds:02}")
}
