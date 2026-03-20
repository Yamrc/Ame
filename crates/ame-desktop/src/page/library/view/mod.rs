mod load;

use nekowg::{App, Context, Entity, Render, Subscription, Window, prelude::*};
use std::rc::Rc;
use std::sync::Arc;

use crate::app::page::PageLifecycle;
use crate::app::route::AppRoute;
use crate::app::router;
use crate::app::runtime::AppRuntime;
use crate::domain::player;
use crate::page::library::models::{LibraryPageSnapshot, LibraryTab};
use crate::page::library::sections::{
    PlaylistActionHandler, PreviewPlayHandler, TabActionHandler, render_library_sections,
};
use crate::page::library::state::LibraryPageState;
use crate::page::playlist;
use crate::page::state::freeze_page_state;

pub struct LibraryPageView {
    runtime: AppRuntime,
    state: Entity<LibraryPageState>,
    observed_user_id: Option<i64>,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

impl LibraryPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| LibraryPageState::default());
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        let observed_user_id = runtime.session.read(cx).auth_user_id;
        Self {
            runtime,
            state,
            observed_user_id,
            active: false,
            _subscriptions: subscriptions,
        }
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
}

impl Render for LibraryPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let library = self.state.read(cx).clone();
        let session = self.runtime.session.read(cx).clone();
        let snapshot = LibraryPageSnapshot::from_state(
            &library.playlists,
            &library.liked_tracks,
            &library.liked_lyric_lines,
            library.tab,
            session.auth_account_summary.as_deref(),
            session.auth_user_name.as_deref(),
            session.auth_user_avatar.as_deref(),
        );
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
            snapshot,
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

    fn on_frozen(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        freeze_page_state(&self.state, cx);
    }
}
