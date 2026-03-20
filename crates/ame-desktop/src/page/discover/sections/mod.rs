mod filters;

use std::rc::Rc;

use nekowg::{AnyElement, App, FontWeight, div, prelude::*, px, rgb};

use crate::component::playlist_card::{self, PlaylistCardActions, PlaylistCardProps};
use crate::component::{page, theme};
use crate::page::discover::models::DiscoverPlaylistCard;

use self::filters::render_filter_row;

const PLAYLIST_GRID_COLUMNS: usize = 5;

pub(crate) type PlaylistOpenHandler = Rc<dyn Fn(i64, &mut App)>;

pub(crate) struct DiscoverSectionsRender<'a> {
    pub loading: bool,
    pub error: Option<&'a str>,
    pub playlists: &'a [DiscoverPlaylistCard],
}

pub(crate) fn render_discover_page(
    view: DiscoverSectionsRender<'_>,
    on_open_playlist: PlaylistOpenHandler,
) -> AnyElement {
    let rows = view
        .playlists
        .iter()
        .map(|item| {
            let playlist_id = item.id;
            let on_open_playlist = on_open_playlist.clone();
            render_playlist_card(item, move |cx| on_open_playlist(playlist_id, cx))
        })
        .collect::<Vec<_>>();
    let status = page::status_banner(view.loading, view.error, "加载中...", "加载失败");
    let playlist_section =
        page::grid_or_empty(rows, PLAYLIST_GRID_COLUMNS, px(18.), "暂无推荐内容");

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(28.))
        .child(
            div()
                .text_size(px(56.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child("发现"),
        )
        .child(render_filter_row())
        .child(status)
        .child(playlist_section)
        .into_any_element()
}

fn render_playlist_card(
    item: &DiscoverPlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
    playlist_card::render(
        PlaylistCardProps::standard(
            item.name.clone(),
            playlist_card::subtitle_with_count(Some(item.track_count), &item.creator_name),
            item.cover_url.clone(),
        ),
        PlaylistCardActions {
            on_open: Some(Rc::new(on_open)),
        },
    )
}
