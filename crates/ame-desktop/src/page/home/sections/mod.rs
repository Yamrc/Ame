mod featured;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{AnyElement, App, div, prelude::*, px};

use crate::component::{
    cover_card::{self, ArtistCoverCardProps, CoverCardActions},
    page,
    playlist_card::{self, PlaylistCardActions, PlaylistCardProps},
    section,
};
use crate::domain::library as library_actions;
use crate::page::home::models::{HomeArtistCard, HomePlaylistCard};

use self::featured::{daily_featured_card, fm_featured_card};

pub(crate) type OpenDailyHandler = Arc<dyn Fn(&mut App)>;
pub(crate) type PlayDailyHandler = Arc<dyn Fn(Option<i64>, &mut App)>;
pub(crate) type OpenFmHandler = Arc<dyn Fn(Option<library_actions::FmTrackItem>, &mut App)>;
pub(crate) type OpenPlaylistHandler = Arc<dyn Fn(i64, &mut App)>;

pub(crate) struct HomeSectionsRender<'a> {
    pub loading: bool,
    pub error: Option<&'a str>,
    pub daily_card: &'a HomePlaylistCard,
    pub daily_first_track_id: Option<i64>,
    pub fm_card: &'a HomePlaylistCard,
    pub fm_track: Option<&'a library_actions::FmTrackItem>,
    pub playlists: &'a [HomePlaylistCard],
    pub artists: &'a [HomeArtistCard],
    pub albums: &'a [HomePlaylistCard],
    pub toplists: &'a [HomePlaylistCard],
}

pub(crate) fn render_home_sections(
    view: HomeSectionsRender<'_>,
    on_open_daily: OpenDailyHandler,
    on_play_daily: PlayDailyHandler,
    on_open_fm: OpenFmHandler,
    on_open_playlist: OpenPlaylistHandler,
) -> AnyElement {
    let daily_first_track_id = view.daily_first_track_id;
    let fm_track = view.fm_track.cloned();
    let featured_rows = vec![
        {
            let on_open_daily = on_open_daily.clone();
            let on_play_daily = on_play_daily.clone();
            daily_featured_card(
                view.daily_card.clone(),
                move |cx| on_open_daily(cx),
                move |cx| on_play_daily(daily_first_track_id, cx),
            )
        },
        {
            let on_open_fm = on_open_fm.clone();
            fm_featured_card(view.fm_card.clone(), move |cx| {
                on_open_fm(fm_track.clone(), cx)
            })
        },
    ];
    let playlist_rows = view
        .playlists
        .iter()
        .map(|item| render_playlist_card(item, on_open_playlist.clone()))
        .collect();
    let artist_rows = view
        .artists
        .iter()
        .map(|artist| {
            cover_card::render_artist_card(
                ArtistCoverCardProps {
                    name: artist.name.clone(),
                    cover_url: artist.cover_url.clone(),
                },
                CoverCardActions::default(),
            )
        })
        .collect();
    let album_rows = view
        .albums
        .iter()
        .map(|item| {
            playlist_card::render(
                PlaylistCardProps::standard(
                    item.name.clone(),
                    item.subtitle.clone(),
                    item.cover_url.clone(),
                ),
                PlaylistCardActions::default(),
            )
        })
        .collect();
    let toplist_rows = view
        .toplists
        .iter()
        .map(|item| {
            playlist_card::render(
                PlaylistCardProps::standard(
                    item.name.clone(),
                    item.subtitle.clone(),
                    item.cover_url.clone(),
                ),
                PlaylistCardActions::default(),
            )
        })
        .collect();

    let status = page::status_banner(view.loading, view.error, "加载中...", "加载失败");

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(28.))
        .child(div().w_full().mt(px(12.)).child(status))
        .child(section::title("For You", None, Some(px(22.))))
        .child(page::grid_or_empty(featured_rows, 2, px(20.), "暂无推荐"))
        .child(section::title("推荐歌单", Some(px(36.)), Some(px(14.))))
        .child(page::grid_or_empty(
            playlist_rows,
            5,
            px(20.),
            "暂无推荐歌单",
        ))
        .child(section::title("推荐艺人", Some(px(40.)), Some(px(14.))))
        .child(page::grid_or_empty(artist_rows, 6, px(20.), "暂无推荐艺人"))
        .child(section::title("新碟上架", Some(px(40.)), Some(px(14.))))
        .child(page::grid_or_empty(album_rows, 5, px(20.), "暂无新碟"))
        .child(section::title("榜单", Some(px(40.)), Some(px(14.))))
        .child(page::grid_or_empty(toplist_rows, 5, px(20.), "暂无榜单"))
        .into_any_element()
}

fn render_playlist_card(
    item: &HomePlaylistCard,
    on_open_playlist: OpenPlaylistHandler,
) -> AnyElement {
    let playlist_id = item.id;
    playlist_card::render(
        PlaylistCardProps::standard(
            item.name.clone(),
            item.subtitle.clone(),
            item.cover_url.clone(),
        ),
        PlaylistCardActions {
            on_open: Some(Rc::new(move |cx| on_open_playlist(playlist_id, cx))),
        },
    )
}
