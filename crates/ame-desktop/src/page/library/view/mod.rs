mod load;

use nekowg::{App, Context, Entity, Render, Subscription, Window, prelude::*};
use std::rc::Rc;
use std::sync::Arc;

use crate::app::page::{PageLifecycle, PageRetentionPolicy, PageSnapshot};
use crate::app::route::AppRoute;
use crate::app::router;
use crate::app::runtime::AppRuntime;
use crate::domain::player;
use crate::page::library::LibraryPageFrozenState;
use crate::page::library::models::{LibraryPlaylistCard, LibraryTab};
use crate::page::library::sections::{
    LibrarySectionsRender, PlaylistActionHandler, PreviewPlayHandler, TabActionHandler,
    render_library_sections,
};
use crate::page::library::state::LibraryPageState;
use crate::page::playlist;
use crate::page::state::freeze_page_state;

pub struct LibraryPageView {
    runtime: AppRuntime,
    state: Entity<LibraryPageState>,
    observed_user_id: Option<i64>,
    heavy_resources: LibraryHeavyResources,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

#[derive(Default)]
struct LibraryHeavyResources {
    title: String,
    user_avatar: Option<String>,
    liked_playlist: Option<LibraryPlaylistCard>,
    preview_tracks: Vec<crate::domain::library::PlaylistTrackItem>,
    liked_lyric_lines: Vec<String>,
    created_playlists: Vec<LibraryPlaylistCard>,
    collected_playlists: Vec<LibraryPlaylistCard>,
    followed_playlists: Vec<LibraryPlaylistCard>,
}

impl LibraryPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| LibraryPageState::default());
        let mut view = Self {
            runtime,
            state,
            observed_user_id: None,
            heavy_resources: LibraryHeavyResources::default(),
            active: false,
            _subscriptions: Vec::new(),
        };
        view.observed_user_id = view.runtime.session.read(cx).auth_user_id;
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&view.state, |this, _, cx| {
            this.refresh_heavy_resources(cx);
            cx.notify();
        }));
        subscriptions.push(cx.observe(&view.runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        view._subscriptions = subscriptions;
        view.refresh_heavy_resources(cx);
        view
    }

    fn set_tab(&mut self, tab: LibraryTab, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.tab = tab;
            cx.notify();
        });
    }

    fn replace_queue_from_playlist(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        let page = match playlist::ensure_playlist_page_loaded(&self.runtime, playlist_id, cx) {
            Ok(page) => page,
            Err(err) => {
                crate::domain::session::push_shell_error(
                    &self.runtime,
                    format!("替换队列失败: {err}"),
                    cx,
                );
                return;
            }
        };
        let tracks = page
            .tracks
            .into_iter()
            .map(player::QueueTrackInput::from)
            .collect::<Vec<_>>();
        player::replace_queue(&self.runtime, tracks, 0, cx);
    }

    fn clear_state(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.clear();
            cx.notify();
        });
    }

    fn refresh_heavy_resources(&mut self, cx: &mut Context<Self>) {
        let library = self.state.read(cx);
        let session = self.runtime.session.read(cx);
        let title = session
            .auth_user_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .map(|name| format!("{name} 的音乐库"))
            .or_else(|| {
                session
                    .auth_account_summary
                    .as_deref()
                    .filter(|summary| !summary.trim().is_empty())
                    .map(|summary| format!("{summary} 的音乐库"))
            })
            .unwrap_or_else(|| "我的音乐库".to_string());
        self.heavy_resources = LibraryHeavyResources {
            title,
            user_avatar: session.auth_user_avatar.clone(),
            liked_playlist: library
                .playlists
                .data
                .iter()
                .find(|item| item.special_type == 5)
                .map(|item| LibraryPlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    track_count: item.track_count,
                    creator_name: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                }),
            preview_tracks: library.liked_tracks.data.iter().take(12).cloned().collect(),
            liked_lyric_lines: library.liked_lyric_lines.iter().take(2).cloned().collect(),
            created_playlists: library
                .playlists
                .data
                .iter()
                .filter(|item| !item.subscribed && item.special_type != 5)
                .map(|item| LibraryPlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    track_count: item.track_count,
                    creator_name: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
            collected_playlists: library
                .playlists
                .data
                .iter()
                .filter(|item| item.subscribed)
                .map(|item| LibraryPlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    track_count: item.track_count,
                    creator_name: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
            followed_playlists: Vec::new(),
        };
    }
}

impl Render for LibraryPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let library = self.state.read(cx);
        let page = cx.entity();
        let on_open_playlist: PlaylistActionHandler = {
            let page = page.clone();
            Rc::new(move |playlist_id, cx| {
                page.update(cx, |_, cx| {
                    router::navigate_route(cx, AppRoute::Playlist { id: playlist_id });
                });
            })
        };
        let on_replace_queue_from_playlist: PlaylistActionHandler = {
            let page = page.clone();
            Rc::new(move |playlist_id, cx| {
                page.update(cx, |this, cx| {
                    this.replace_queue_from_playlist(playlist_id, cx)
                });
            })
        };
        let on_preview_play: PreviewPlayHandler = {
            let page = page.clone();
            Arc::new(move |track, cx| {
                page.update(cx, |this, cx| {
                    player::enqueue_track(&this.runtime, track.into(), true, cx);
                });
            })
        };
        let on_tab_created: TabActionHandler = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.set_tab(LibraryTab::Created, cx));
            })
        };
        let on_tab_collected: TabActionHandler = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.set_tab(LibraryTab::Collected, cx));
            })
        };
        let on_tab_followed: TabActionHandler = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.set_tab(LibraryTab::Followed, cx));
            })
        };

        render_library_sections(
            LibrarySectionsRender {
                loading: library.playlists.loading,
                error: library.playlists.error.as_deref(),
                liked_playlist: self.heavy_resources.liked_playlist.as_ref(),
                liked_tracks: &self.heavy_resources.preview_tracks,
                liked_lyric_lines: &self.heavy_resources.liked_lyric_lines,
                created_playlists: &self.heavy_resources.created_playlists,
                collected_playlists: &self.heavy_resources.collected_playlists,
                followed_playlists: &self.heavy_resources.followed_playlists,
                active_tab: library.tab,
                title: &self.heavy_resources.title,
                user_avatar: self.heavy_resources.user_avatar.as_deref(),
            },
            on_open_playlist,
            on_replace_queue_from_playlist,
            on_preview_play,
            on_tab_created,
            on_tab_collected,
            on_tab_followed,
        )
    }
}

impl PageLifecycle for LibraryPageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::SnapshotOnly
    }

    fn capture_snapshot(&mut self, cx: &mut Context<Self>) -> Option<PageSnapshot> {
        Some(PageSnapshot::Library(self.state.read(cx).frozen_state()))
    }

    fn restore_snapshot(
        &mut self,
        snapshot: PageSnapshot,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let snapshot = match snapshot {
            PageSnapshot::Library(snapshot) => snapshot,
            PageSnapshot::__Reserved => {
                return Err("收到保留的音乐库页面快照类型".to_string());
            }
        };
        self.state.update(cx, |state, cx| {
            state.restore_frozen_state(LibraryPageFrozenState { tab: snapshot.tab });
            cx.notify();
        });
        Ok(())
    }

    fn release_view_resources(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        self.heavy_resources = LibraryHeavyResources::default();
        freeze_page_state(&self.state, cx);
    }
}
