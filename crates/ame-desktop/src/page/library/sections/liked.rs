use std::rc::Rc;

use nekowg::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, relative, rgb};

use crate::component::short_track_item::{self, ShortTrackItemActions, ShortTrackItemProps};
use crate::component::{icon, page, theme};
use crate::domain::library::PlaylistTrackItem;
use crate::page::library::models::LibraryPlaylistCard;

use super::{PREVIEW_COLS, PREVIEW_MAX, PreviewPlayHandler};

pub(super) fn liked_card(
    item: LibraryPlaylistCard,
    lyric_lines: &[String],
    min_height: nekowg::Pixels,
    on_open: impl Fn(&mut App) + 'static,
    on_play: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let top_lines = if lyric_lines.is_empty() {
        vec!["暂无喜欢歌曲".to_string()]
    } else {
        lyric_lines.iter().take(2).cloned().collect()
    };

    let play_button = div()
        .size(px(44.))
        .rounded_full()
        .bg(rgb(theme::COLOR_PRIMARY))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .child(icon::render(
            icon::IconName::Play,
            16.,
            theme::COLOR_PRIMARY_BG_DARK,
        ))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            on_play(cx);
        });

    div()
        .w(relative(0.330))
        .cursor_pointer()
        .rounded_2xl()
        .px(px(24.))
        .py(px(18.))
        .bg(rgb(theme::COLOR_PRIMARY_BG_DARK))
        .min_h(min_height)
        .flex()
        .flex_col()
        .justify_between()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .text_size(px(14.))
                .line_height(relative(1.2))
                .font_weight(FontWeight::LIGHT)
                .text_color(rgb(theme::COLOR_PRIMARY))
                .children(
                    top_lines
                        .into_iter()
                        .map(|line| div().child(line).into_any_element()),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(24.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(theme::COLOR_PRIMARY))
                                .line_height(relative(1.2))
                                .child(item.name),
                        )
                        .child(
                            div()
                                .text_size(px(15.))
                                .mt(px(2.))
                                .line_height(relative(1.2))
                                .text_color(rgb(theme::COLOR_PRIMARY))
                                .child(format!("{} 首歌", item.track_count)),
                        ),
                )
                .child(play_button),
        )
        .into_any_element()
}

pub(super) fn empty_liked_card(min_height: nekowg::Pixels) -> AnyElement {
    div()
        .w(relative(0.330))
        .min_h(min_height)
        .child(page::empty_card("暂无喜欢的音乐"))
        .into_any_element()
}

pub(super) fn liked_preview_list(
    tracks: &[PlaylistTrackItem],
    row_height: f32,
    row_gap: f32,
    on_play: PreviewPlayHandler,
) -> AnyElement {
    if tracks.is_empty() {
        return page::empty_card("暂无喜欢歌曲");
    }

    div()
        .overflow_hidden()
        .grid()
        .grid_cols(PREVIEW_COLS as u16)
        .gap(px(row_gap))
        .children(
            tracks
                .iter()
                .take(PREVIEW_MAX)
                .enumerate()
                .map(|(index, track)| {
                    let track_for_play = track.clone();
                    let on_play = on_play.clone();
                    short_track_item::render(
                        ShortTrackItemProps {
                            id: track.id,
                            state_id: format!("library-liked-preview:{index}:track:{}", track.id)
                                .into(),
                            title: track.name.clone(),
                            subtitle: track.artists.clone(),
                            cover_url: track.cover_url.clone(),
                            height: px(row_height),
                        },
                        ShortTrackItemActions {
                            on_play: Some(Rc::new(move |cx| on_play(track_for_play.clone(), cx))),
                            ..ShortTrackItemActions::default()
                        },
                    )
                }),
        )
        .into_any_element()
}
