mod detail;
mod lyric;
mod models;
mod parse;
mod playlists;
mod recommendations;

pub use detail::fetch_playlist_detail_blocking;
pub use lyric::fetch_track_lyric_preview_blocking;
pub use models::*;
pub use playlists::{
    fetch_daily_recommend_playlists_blocking, fetch_daily_recommend_tracks_blocking,
    fetch_personal_fm_blocking, fetch_personalized_playlists_blocking,
    fetch_top_playlists_blocking, fetch_user_playlists_blocking,
};
pub use recommendations::{
    fetch_new_albums_blocking, fetch_recommend_artists_blocking, fetch_toplists_blocking,
};
