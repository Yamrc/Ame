mod factory;
mod lifecycle;
mod unknown;

use std::time::Instant;

use nekowg::{Entity, Pixels};

use crate::app::page::PageSnapshot;
use crate::page::{daily_tracks, discover, home, library, login, next, playlist, search, settings};

use super::key::PageKey;

pub(in crate::app::page_host) use factory::create_page;
use unknown::UnknownPageView;

pub(super) enum PageSlot {
    Home(Entity<home::HomePageView>),
    Discover(Entity<discover::DiscoverPageView>),
    Library(Entity<library::LibraryPageView>),
    Search(Entity<search::SearchPageView>),
    DailyTracks(Entity<daily_tracks::DailyTracksPageView>),
    Next(Entity<next::NextPageView>),
    Playlist(Entity<playlist::PlaylistPageView>),
    Settings(Entity<settings::SettingsPageView>),
    Login(Entity<login::LoginPageView>),
    Unknown(Entity<UnknownPageView>),
}

pub(super) struct PageInstance {
    pub(super) key: PageKey,
    pub(super) slot: PageSlot,
    pub(super) scroll_offset: Pixels,
}

pub(super) struct FrozenPage {
    pub(super) slot: PageSlot,
    pub(super) destroy_at: Instant,
    pub(super) scroll_offset: Pixels,
}

pub(super) struct FrozenSnapshot {
    pub(super) snapshot: Option<PageSnapshot>,
    pub(super) destroy_at: Instant,
    pub(super) scroll_offset: Pixels,
}

pub(super) enum FrozenEntry {
    KeepAlive(FrozenPage),
    Snapshot(Box<FrozenSnapshot>),
}
