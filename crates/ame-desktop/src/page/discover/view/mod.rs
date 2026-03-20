mod load;

use std::rc::Rc;

use nekowg::{Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::route::AppRoute;
use crate::app::router;
use crate::app::runtime::AppRuntime;
use crate::page::discover::models::DiscoverPlaylistCard;
use crate::page::discover::sections::{
    DiscoverSectionsRender, PlaylistOpenHandler, render_discover_page,
};
use crate::page::discover::state::DiscoverPageState;
use crate::page::state::freeze_page_state;

pub struct DiscoverPageView {
    runtime: AppRuntime,
    state: Entity<DiscoverPageState>,
    last_user_token_state: bool,
    heavy_resources: DiscoverHeavyResources,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

#[derive(Default)]
struct DiscoverHeavyResources {
    playlists: Vec<DiscoverPlaylistCard>,
}

impl DiscoverPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| DiscoverPageState::default());
        let mut view = Self {
            runtime,
            state,
            last_user_token_state: false,
            heavy_resources: DiscoverHeavyResources::default(),
            active: false,
            _subscriptions: Vec::new(),
        };
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&view.state, |this, _, cx| {
            this.refresh_heavy_resources(cx);
            cx.notify();
        }));
        subscriptions.push(cx.observe(&view.runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        view.last_user_token_state = view
            .runtime
            .session
            .read(cx)
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        view._subscriptions = subscriptions;
        view.refresh_heavy_resources(cx);
        view
    }

    fn clear_state(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.clear();
            cx.notify();
        });
    }

    fn refresh_heavy_resources(&mut self, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        self.heavy_resources = DiscoverHeavyResources {
            playlists: state
                .playlists
                .data
                .iter()
                .take(12)
                .map(|item| DiscoverPlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    track_count: item.track_count,
                    creator_name: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
        };
    }
}

impl Render for DiscoverPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let page = cx.entity();
        let on_open_playlist: PlaylistOpenHandler = Rc::new(move |playlist_id, cx| {
            page.update(cx, |_, cx| {
                router::navigate_route(cx, AppRoute::Playlist { id: playlist_id });
            });
        });

        render_discover_page(
            DiscoverSectionsRender {
                loading: state.playlists.loading,
                error: state.playlists.error.as_deref(),
                playlists: &self.heavy_resources.playlists,
            },
            on_open_playlist,
        )
    }
}

impl PageLifecycle for DiscoverPageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::SnapshotOnly
    }

    fn release_view_resources(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        self.heavy_resources = DiscoverHeavyResources::default();
        freeze_page_state(&self.state, cx);
    }
}
