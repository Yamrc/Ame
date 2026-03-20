mod load;

use std::rc::Rc;

use nekowg::{Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::PageLifecycle;
use crate::app::route::AppRoute;
use crate::app::router;
use crate::app::runtime::AppRuntime;
use crate::page::discover::models::DiscoverPageSnapshot;
use crate::page::discover::sections::{PlaylistOpenHandler, render_discover_page};
use crate::page::discover::state::DiscoverPageState;
use crate::page::state::freeze_page_state;

pub struct DiscoverPageView {
    runtime: AppRuntime,
    state: Entity<DiscoverPageState>,
    last_user_token_state: bool,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

impl DiscoverPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| DiscoverPageState::default());
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        let last_user_token_state = runtime
            .session
            .read(cx)
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        Self {
            runtime,
            state,
            last_user_token_state,
            active: false,
            _subscriptions: subscriptions,
        }
    }

    fn clear_state(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.clear();
            cx.notify();
        });
    }
}

impl Render for DiscoverPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = DiscoverPageSnapshot::from_state(&self.state.read(cx).playlists);
        let page = cx.entity();
        let on_open_playlist: PlaylistOpenHandler = Rc::new(move |playlist_id, cx| {
            page.update(cx, |_, cx| {
                router::navigate_route(cx, AppRoute::Playlist { id: playlist_id });
            });
        });

        render_discover_page(snapshot, on_open_playlist)
    }
}

impl PageLifecycle for DiscoverPageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn on_frozen(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        freeze_page_state(&self.state, cx);
    }
}
