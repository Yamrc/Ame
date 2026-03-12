use crate::router::navigate;
use nekowg::{Context, PromptButton, PromptLevel, SharedString};

use ame_audio::{AudioCommand, AudioError, SeekTarget, SourceSpec};
use nekowg::{Image, ImageFormat};
use qrcode::{QrCode, render::svg};
use rand::RngExt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::action::{auth_actions, library_actions, player_actions, queue_actions, search_actions};
use crate::entity::app::CloseBehavior;
use crate::entity::player::QueueItem;
use crate::kernel::{AppCommand, AppEvent, KernelCommandSender, SongInput};
use crate::view::{playlist, search};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::{
    KEY_PLAYER_CURRENT_INDEX, KEY_PLAYER_DURATION_MS, KEY_PLAYER_MODE, KEY_PLAYER_POSITION_MS,
    KEY_PLAYER_QUEUE, KEY_PLAYER_VOLUME, KEY_PLAYER_WAS_PLAYING, KEY_WINDOW_CLOSE_BEHAVIOR,
    PROGRESS_PERSIST_INTERVAL, QR_POLL_INTERVAL, QR_POLL_TIMEOUT, RootView,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthLevel {
    Guest,
    User,
}

const HOME_RECOMMEND_TTL: Duration = Duration::from_secs(30 * 60);
const HOME_DAILY_TRACKS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const HOME_PERSONAL_FM_TTL: Duration = Duration::from_secs(3 * 60);
const HOME_RECOMMEND_ARTISTS_TTL: Duration = Duration::from_secs(12 * 60 * 60);
const HOME_NEW_ALBUMS_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const HOME_TOPLIST_TTL: Duration = Duration::from_secs(12 * 60 * 60);
const DISCOVER_PLAYLIST_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const LIBRARY_PLAYLIST_TTL: Duration = Duration::from_secs(2 * 60);
const PLAYLIST_DETAIL_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    fetched_at_ms: u64,
    data: T,
}

impl RootView {
    pub(crate) fn queue_kernel_command(&mut self, command: AppCommand) {
        let _ = self.kernel_runtime.command_sender().send(command);
    }

    pub(crate) fn kernel_command_sender(&self) -> KernelCommandSender {
        self.kernel_runtime.command_sender()
    }

    pub(crate) fn drain_kernel_events(&mut self, cx: &mut Context<Self>) -> bool {
        let mut changed = false;
        while let Some(event) = self.kernel_runtime.try_recv_event() {
            let AppEvent::Command(command) = event;
            self.apply_kernel_command(command, cx);
            changed = true;
        }
        changed
    }

    fn apply_kernel_command(&mut self, command: AppCommand, cx: &mut Context<Self>) {
        match command {
            AppCommand::Navigate(path) => Self::navigate_to(path, cx),
            AppCommand::SubmitSearchFromQuery => self.submit_search_from_query(cx),
            AppCommand::GenerateLoginQr => self.generate_login_qr(cx),
            AppCommand::StopLoginQrPolling => self.stop_login_qr_polling(cx),
            AppCommand::EnsureGuestSession => self.ensure_guest_session(),
            AppCommand::RefreshLoginToken => self.refresh_login_token(),
            AppCommand::SetCloseBehavior(value) => self.set_close_behavior(value, cx),
            AppCommand::OpenLibraryPlaylist(playlist_id) => {
                self.open_playlist_from_library(playlist_id, cx)
            }
            AppCommand::ReplaceQueueFromPlaylist(playlist_id) => {
                self.replace_queue_from_playlist(playlist_id, cx)
            }
            AppCommand::ReplaceQueueFromDailyTracks(track_id) => {
                self.replace_queue_from_daily_tracks(track_id, cx)
            }
            AppCommand::EnqueueSongAndPlay(song) => {
                self.enqueue_song_from_route(song_input_to_search_song(song), cx)
            }
            AppCommand::EnqueueSongOnly(song) => {
                self.enqueue_song_without_play_from_route(song_input_to_search_song(song), cx)
            }
            AppCommand::PlayQueueItem(track_id) => self.play_queue_item_from_route(track_id, cx),
            AppCommand::RemoveQueueItem(track_id) => {
                self.remove_queue_item_from_route(track_id, cx)
            }
            AppCommand::ClearQueue => self.clear_queue_from_route(cx),
            AppCommand::PreviousTrack => self.play_previous(cx),
            AppCommand::TogglePlay => self.toggle_playback(cx),
            AppCommand::NextTrack => self.play_next(cx),
            AppCommand::CyclePlayMode => self.cycle_play_mode(cx),
            AppCommand::Quit => {
                self.prepare_app_exit(cx);
                cx.quit();
            }
            AppCommand::Shutdown => {}
        }
    }

    fn has_user_token(&self) -> bool {
        self.auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }

    fn has_guest_token(&self) -> bool {
        self.has_user_token()
            || self
                .auth_bundle
                .music_a
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    fn ensure_guest_token(&mut self) -> bool {
        if self.has_guest_token() {
            return true;
        }

        let current_cookie = auth_actions::build_cookie_header(&self.auth_bundle);
        match auth_actions::register_anonymous_blocking(current_cookie.as_deref()) {
            Ok(response) => {
                self.merge_auth_cookies(&response.set_cookie);
                if self
                    .auth_bundle
                    .music_a
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    true
                } else {
                    Self::push_error(
                        &mut self.search_error,
                        "游客登录返回成功但未拿到 MUSIC_A".to_string(),
                    );
                    false
                }
            }
            Err(err) => {
                Self::push_error(&mut self.search_error, format!("游客登录失败: {err}"));
                false
            }
        }
    }

    fn ensure_auth_cookie(&mut self, level: AuthLevel) -> Option<String> {
        let ok = match level {
            AuthLevel::Guest => self.ensure_guest_token(),
            AuthLevel::User => {
                if self.has_user_token() {
                    true
                } else {
                    Self::push_error(
                        &mut self.search_error,
                        "当前操作需要账号登录凭据(MUSIC_U)".to_string(),
                    );
                    false
                }
            }
        };
        if !ok {
            return None;
        }

        let cookie = auth_actions::build_cookie_header(&self.auth_bundle);
        if cookie.is_none() {
            Self::push_error(
                &mut self.search_error,
                "鉴权凭据异常，已阻止请求".to_string(),
            );
        }
        cookie
    }

    fn merge_auth_cookies(&mut self, set_cookie: &[String]) -> bool {
        let changed = auth_actions::merge_bundle_from_set_cookie(&mut self.auth_bundle, set_cookie);
        if changed {
            self.persist_auth_bundle();
        }
        changed
    }

    fn persist_auth_bundle(&mut self) {
        if let Err(err) = self.credential_store.save_auth_bundle(&self.auth_bundle) {
            Self::push_error(
                &mut self.search_error,
                format!("写入 keyring 凭据失败: {err}"),
            );
        }
    }

    pub(super) fn persist_player_settings(&mut self, cx: &mut Context<Self>) {
        let Some(settings) = self.settings_store.as_ref() else {
            return;
        };
        let player = self.player.read(cx).clone();
        if let Err(err) = settings.set(KEY_PLAYER_VOLUME, &player.volume) {
            Self::push_error(&mut self.search_error, format!("保存音量失败: {err}"));
        }
        if let Err(err) = settings.set(KEY_PLAYER_MODE, &player.mode) {
            Self::push_error(&mut self.search_error, format!("保存播放模式失败: {err}"));
        }
        if let Err(err) = settings.set(KEY_WINDOW_CLOSE_BEHAVIOR, &self.close_behavior) {
            Self::push_error(&mut self.search_error, format!("保存关闭行为失败: {err}"));
        }
    }

    pub(super) fn persist_player_runtime(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.state_store.as_ref() else {
            return;
        };
        let player = self.player.read(cx).clone();
        let queue = player
            .queue
            .iter()
            .map(|item| super::PersistedQueueItem {
                id: item.id,
                name: item.name.clone(),
                artist: item.artist.clone(),
                cover_url: item.cover_url.clone(),
            })
            .collect::<Vec<_>>();

        if let Err(err) = state.set(KEY_PLAYER_QUEUE, &queue) {
            Self::push_error(&mut self.search_error, format!("保存队列失败: {err}"));
        }
        if let Err(err) = state.set(KEY_PLAYER_CURRENT_INDEX, &player.current_index) {
            Self::push_error(&mut self.search_error, format!("保存当前索引失败: {err}"));
        }
        if let Err(err) = state.set(KEY_PLAYER_WAS_PLAYING, &player.is_playing) {
            Self::push_error(&mut self.search_error, format!("保存播放状态失败: {err}"));
        }
    }

    fn persist_player_progress(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.state_store.as_ref() else {
            return;
        };
        let player = self.player.read(cx).clone();
        if let Err(err) = state.set(KEY_PLAYER_POSITION_MS, &player.position_ms) {
            Self::push_error(&mut self.search_error, format!("保存播放进度失败: {err}"));
        }
        if let Err(err) = state.set(KEY_PLAYER_DURATION_MS, &player.duration_ms) {
            Self::push_error(&mut self.search_error, format!("保存播放时长失败: {err}"));
        }
    }

    pub(super) fn persist_progress_by_interval(
        &mut self,
        now: std::time::Instant,
        cx: &mut Context<Self>,
    ) {
        if now.duration_since(self.last_progress_persist_at) < PROGRESS_PERSIST_INTERVAL {
            return;
        }
        self.last_progress_persist_at = now;
        self.persist_player_progress(cx);
    }

    fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default()
    }

    fn cache_is_fresh(fetched_at_ms: u64, ttl: Duration) -> bool {
        let ttl_ms = ttl.as_millis() as u64;
        let now_ms = Self::now_millis();
        now_ms.saturating_sub(fetched_at_ms) <= ttl_ms
    }

    fn read_cache<T: DeserializeOwned>(&self, key: &str, ttl: Duration) -> Option<CacheEntry<T>> {
        let store = self.cache_store.as_ref()?;
        let entry: CacheEntry<T> = store.get(key).ok().flatten()?;
        if Self::cache_is_fresh(entry.fetched_at_ms, ttl) {
            Some(entry)
        } else {
            None
        }
    }

    fn write_cache<T: Serialize>(&self, key: &str, data: &T) -> Option<u64> {
        let store = self.cache_store.as_ref()?;
        let fetched_at_ms = Self::now_millis();
        let entry = CacheEntry {
            fetched_at_ms,
            data,
        };
        if store.upsert(key, &entry).is_ok() {
            Some(fetched_at_ms)
        } else {
            None
        }
    }

    pub(super) fn refresh_login_summary(&mut self) {
        if self.auth_bundle.music_u.is_none() {
            self.auth_account_summary = None;
            self.auth_user_name = None;
            self.auth_user_avatar = None;
            self.auth_user_id = None;
            self.library_playlists.data.clear();
            self.library_playlists.error = None;
            self.library_playlists.loading = false;
            self.playlist_state.data.clear();
            self.playlist_state.error = None;
            self.library_liked_lyric_lines.clear();
            self.daily_tracks.data.clear();
            self.daily_tracks.error = None;
            self.daily_tracks.loading = false;
            self.personal_fm.data = None;
            self.personal_fm.error = None;
            self.personal_fm.loading = false;
            self.refresh_home_data();
            return;
        }
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            self.auth_account_summary = None;
            self.auth_user_name = None;
            self.auth_user_avatar = None;
            self.auth_user_id = None;
            return;
        };
        match auth_actions::fetch_login_status_blocking(Some(cookie.as_str())) {
            Ok(body) => {
                let data = body.get("data").unwrap_or(&body);
                let profile = data
                    .get("profile")
                    .or_else(|| body.get("profile"))
                    .unwrap_or(&serde_json::Value::Null);
                self.auth_account_summary = auth_actions::login_summary_text(&body);
                self.auth_user_name = profile["nickname"].as_str().map(str::to_string);
                self.auth_user_avatar = profile["avatarUrl"].as_str().map(str::to_string);
                self.auth_user_id = data["account"]["id"]
                    .as_i64()
                    .or_else(|| body["account"]["id"].as_i64())
                    .or_else(|| profile["userId"].as_i64());
                if self.auth_user_id.is_some() {
                    self.refresh_library_playlists();
                } else {
                    self.library_playlists.data.clear();
                    self.library_playlists.error = Some("读取用户信息失败".to_string());
                    self.library_playlists.loading = false;
                }
                self.refresh_home_data();
                self.refresh_discover_playlists();
            }
            Err(err) => {
                self.auth_account_summary = None;
                self.auth_user_name = None;
                self.auth_user_avatar = None;
                self.auth_user_id = None;
                Self::push_error(&mut self.search_error, format!("读取登录状态失败: {err}"));
            }
        }
    }

    pub(super) fn refresh_library_playlists(&mut self) {
        let Some(user_id) = self.auth_user_id else {
            self.library_playlists.data.clear();
            self.library_playlists.error = None;
            self.library_playlists.loading = false;
            return;
        };

        if let Some(fetched_at_ms) = self.library_playlists.fetched_at_ms
            && Self::cache_is_fresh(fetched_at_ms, LIBRARY_PLAYLIST_TTL)
        {
            return;
        }

        self.library_playlists.loading = true;
        self.library_playlists.error = None;
        self.library_playlists.source = super::DataSource::User;
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            self.library_playlists.loading = false;
            self.library_playlists.error = Some("缺少鉴权凭据".to_string());
            return;
        };
        match library_actions::fetch_user_playlists_blocking(user_id, cookie.as_str()) {
            Ok(items) => {
                self.library_playlists.data = items;
                self.library_playlists.fetched_at_ms = Some(Self::now_millis());
            }
            Err(err) => {
                self.library_playlists.data.clear();
                self.library_playlists.error = Some(err.to_string());
            }
        }
        if let Some(liked_id) = self
            .library_playlists
            .data
            .iter()
            .find(|item| item.special_type == 5)
            .map(|item| item.id)
        {
            self.refresh_library_liked_tracks(liked_id);
        } else {
            self.library_liked_tracks.data.clear();
            self.library_liked_tracks.error = None;
            self.library_liked_tracks.loading = false;
            self.library_liked_tracks.fetched_at_ms = None;
            self.library_liked_lyric_lines.clear();
        }
        self.library_playlists.loading = false;
    }

    fn refresh_library_liked_tracks(&mut self, playlist_id: i64) {
        self.library_liked_tracks.loading = true;
        self.library_liked_tracks.error = None;
        self.library_liked_tracks.source = super::DataSource::User;
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            self.library_liked_tracks.loading = false;
            self.library_liked_tracks.error = Some("缺少鉴权凭据".to_string());
            return;
        };
        match library_actions::fetch_playlist_detail_blocking(playlist_id, cookie.as_str()) {
            Ok(detail) => {
                let tracks = detail.tracks;
                self.library_liked_tracks.data = tracks.clone().into_iter().take(12).collect();
                self.library_liked_tracks.fetched_at_ms = Some(Self::now_millis());
                self.library_liked_lyric_lines.clear();
                if !tracks.is_empty() {
                    let mut rng = rand::rng();
                    let index = rng.random_range(0..tracks.len());
                    let track_id = tracks[index].id;
                    match library_actions::fetch_track_lyric_preview_blocking(
                        track_id,
                        cookie.as_str(),
                    ) {
                        Ok(lines) => {
                            self.library_liked_lyric_lines = lines;
                        }
                        Err(_) => {
                            self.library_liked_lyric_lines.clear();
                        }
                    }
                }
            }
            Err(err) => {
                self.library_liked_tracks.data.clear();
                self.library_liked_tracks.error = Some(err.to_string());
                self.library_liked_lyric_lines.clear();
            }
        }
        self.library_liked_tracks.loading = false;
    }

    pub(super) fn refresh_home_data(&mut self) {
        self.refresh_home_recommend_playlists();
        self.refresh_home_recommend_artists();
        self.refresh_home_new_albums();
        self.refresh_home_toplists();
        if self.has_user_token() {
            self.refresh_daily_tracks();
            self.refresh_personal_fm();
        } else {
            self.daily_tracks.data.clear();
            self.daily_tracks.error = None;
            self.daily_tracks.loading = false;
            self.personal_fm.data = None;
            self.personal_fm.error = None;
            self.personal_fm.loading = false;
        }
    }

    fn refresh_home_recommend_playlists(&mut self) {
        self.home_recommend_playlists.loading = true;
        self.home_recommend_playlists.error = None;
        let Some(guest_cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            self.home_recommend_playlists.loading = false;
            self.home_recommend_playlists.error = Some("缺少鉴权凭据".to_string());
            return;
        };

        let is_user = self.has_user_token();
        let mut cache_key = if is_user {
            self.auth_user_id
                .map(|user_id| format!("home.recommend_playlists.user.{user_id}"))
        } else {
            Some("home.recommend_playlists.guest".to_string())
        };
        if let Some(key) = cache_key.as_deref()
            && let Some(entry) = self.read_cache(key, HOME_RECOMMEND_TTL)
        {
            self.home_recommend_playlists.data = entry.data;
            self.home_recommend_playlists.fetched_at_ms = Some(entry.fetched_at_ms);
            self.home_recommend_playlists.loading = false;
            self.home_recommend_playlists.source = if is_user {
                super::DataSource::User
            } else {
                super::DataSource::Guest
            };
            return;
        }

        let limit = 10;
        let mut merged = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut use_cookie = guest_cookie;
        if is_user {
            if let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) {
                use_cookie = cookie;
            } else {
                cache_key = None;
            }
        }

        if is_user
            && let Ok(recommend) =
                library_actions::fetch_daily_recommend_playlists_blocking(use_cookie.as_str())
        {
            for item in recommend {
                if seen.insert(item.id) {
                    merged.push(item);
                }
            }
        }
        match library_actions::fetch_personalized_playlists_blocking(limit, use_cookie.as_str()) {
            Ok(items) => {
                for item in items {
                    if seen.insert(item.id) {
                        merged.push(item);
                    }
                }
            }
            Err(err) => {
                self.home_recommend_playlists.error = Some(err.to_string());
            }
        }

        if merged.len() > limit as usize {
            merged.truncate(limit as usize);
        }
        self.home_recommend_playlists.data = merged;
        if let Some(key) = cache_key.as_deref()
            && let Some(fetched_at_ms) = self.write_cache(key, &self.home_recommend_playlists.data)
        {
            self.home_recommend_playlists.fetched_at_ms = Some(fetched_at_ms);
        }
        self.home_recommend_playlists.loading = false;
        self.home_recommend_playlists.source = if is_user {
            super::DataSource::User
        } else {
            super::DataSource::Guest
        };
    }

    fn refresh_home_recommend_artists(&mut self) {
        self.home_recommend_artists.loading = true;
        self.home_recommend_artists.error = None;
        let is_user = self.has_user_token();
        let mut cache_key = if is_user {
            self.auth_user_id
                .map(|user_id| format!("home.recommend_artists.user.{user_id}.6"))
        } else {
            Some("home.recommend_artists.guest.6".to_string())
        };

        if let Some(key) = cache_key.as_deref()
            && let Some(entry) = self.read_cache(key, HOME_RECOMMEND_ARTISTS_TTL)
        {
            self.home_recommend_artists.data = entry.data;
            self.home_recommend_artists.fetched_at_ms = Some(entry.fetched_at_ms);
            self.home_recommend_artists.loading = false;
            self.home_recommend_artists.source = if is_user {
                super::DataSource::User
            } else {
                super::DataSource::Guest
            };
            return;
        }

        let cookie = if is_user {
            if let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) {
                cookie
            } else {
                cache_key = None;
                let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                    self.home_recommend_artists.loading = false;
                    self.home_recommend_artists.error = Some("缺少鉴权凭据".to_string());
                    return;
                };
                cookie
            }
        } else {
            let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                self.home_recommend_artists.loading = false;
                self.home_recommend_artists.error = Some("缺少鉴权凭据".to_string());
                return;
            };
            cookie
        };

        match library_actions::fetch_recommend_artists_blocking(1, 6, cookie.as_str()) {
            Ok(items) => {
                self.home_recommend_artists.data = items;
                if let Some(key) = cache_key.as_deref()
                    && let Some(fetched_at_ms) =
                        self.write_cache(key, &self.home_recommend_artists.data)
                {
                    self.home_recommend_artists.fetched_at_ms = Some(fetched_at_ms);
                }
            }
            Err(err) => {
                self.home_recommend_artists.error = Some(err.to_string());
                self.home_recommend_artists.data.clear();
            }
        }
        self.home_recommend_artists.loading = false;
    }

    fn refresh_home_new_albums(&mut self) {
        self.home_new_albums.loading = true;
        self.home_new_albums.error = None;
        let is_user = self.has_user_token();
        let mut cache_key = if is_user {
            self.auth_user_id
                .map(|user_id| format!("home.new_albums.user.{user_id}"))
        } else {
            Some("home.new_albums.guest".to_string())
        };

        if let Some(key) = cache_key.as_deref()
            && let Some(entry) = self.read_cache(key, HOME_NEW_ALBUMS_TTL)
        {
            self.home_new_albums.data = entry.data;
            self.home_new_albums.fetched_at_ms = Some(entry.fetched_at_ms);
            self.home_new_albums.loading = false;
            self.home_new_albums.source = if is_user {
                super::DataSource::User
            } else {
                super::DataSource::Guest
            };
            return;
        }

        let cookie = if is_user {
            if let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) {
                cookie
            } else {
                cache_key = None;
                let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                    self.home_new_albums.loading = false;
                    self.home_new_albums.error = Some("缺少鉴权凭据".to_string());
                    return;
                };
                cookie
            }
        } else {
            let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                self.home_new_albums.loading = false;
                self.home_new_albums.error = Some("缺少鉴权凭据".to_string());
                return;
            };
            cookie
        };

        match library_actions::fetch_new_albums_blocking(10, 0, "ALL", cookie.as_str()) {
            Ok(items) => {
                self.home_new_albums.data = items;
                if let Some(key) = cache_key.as_deref()
                    && let Some(fetched_at_ms) = self.write_cache(key, &self.home_new_albums.data)
                {
                    self.home_new_albums.fetched_at_ms = Some(fetched_at_ms);
                }
            }
            Err(err) => {
                self.home_new_albums.error = Some(err.to_string());
                self.home_new_albums.data.clear();
            }
        }
        self.home_new_albums.loading = false;
    }

    fn refresh_home_toplists(&mut self) {
        self.home_toplists.loading = true;
        self.home_toplists.error = None;
        let is_user = self.has_user_token();
        let mut cache_key = if is_user {
            self.auth_user_id
                .map(|user_id| format!("home.toplists.user.{user_id}"))
        } else {
            Some("home.toplists.guest".to_string())
        };

        if let Some(key) = cache_key.as_deref()
            && let Some(entry) = self.read_cache(key, HOME_TOPLIST_TTL)
        {
            self.home_toplists.data = entry.data;
            self.home_toplists.fetched_at_ms = Some(entry.fetched_at_ms);
            self.home_toplists.loading = false;
            self.home_toplists.source = if is_user {
                super::DataSource::User
            } else {
                super::DataSource::Guest
            };
            return;
        }

        let cookie = if is_user {
            if let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) {
                cookie
            } else {
                cache_key = None;
                let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                    self.home_toplists.loading = false;
                    self.home_toplists.error = Some("缺少鉴权凭据".to_string());
                    return;
                };
                cookie
            }
        } else {
            let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                self.home_toplists.loading = false;
                self.home_toplists.error = Some("缺少鉴权凭据".to_string());
                return;
            };
            cookie
        };

        match library_actions::fetch_toplists_blocking(cookie.as_str()) {
            Ok(items) => {
                self.home_toplists.data = items;
                if let Some(key) = cache_key.as_deref()
                    && let Some(fetched_at_ms) = self.write_cache(key, &self.home_toplists.data)
                {
                    self.home_toplists.fetched_at_ms = Some(fetched_at_ms);
                }
            }
            Err(err) => {
                self.home_toplists.error = Some(err.to_string());
                self.home_toplists.data.clear();
            }
        }
        self.home_toplists.loading = false;
    }

    fn refresh_daily_tracks(&mut self) {
        if let Some(fetched_at_ms) = self.daily_tracks.fetched_at_ms
            && Self::cache_is_fresh(fetched_at_ms, HOME_DAILY_TRACKS_TTL)
        {
            return;
        }
        self.daily_tracks.loading = true;
        self.daily_tracks.error = None;
        self.daily_tracks.source = super::DataSource::User;
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            self.daily_tracks.loading = false;
            self.daily_tracks.error = Some("缺少鉴权凭据".to_string());
            return;
        };
        let cache_key = self
            .auth_user_id
            .map(|user_id| format!("home.daily_tracks.user.{user_id}"));
        if let Some(key) = cache_key.as_deref()
            && let Some(entry) = self.read_cache(key, HOME_DAILY_TRACKS_TTL)
        {
            self.daily_tracks.data = entry.data;
            self.daily_tracks.fetched_at_ms = Some(entry.fetched_at_ms);
            self.daily_tracks.loading = false;
            return;
        }
        match library_actions::fetch_daily_recommend_tracks_blocking(cookie.as_str()) {
            Ok(items) => {
                self.daily_tracks.data = items;
                if let Some(key) = cache_key.as_deref() {
                    if let Some(fetched_at_ms) = self.write_cache(key, &self.daily_tracks.data) {
                        self.daily_tracks.fetched_at_ms = Some(fetched_at_ms);
                    }
                } else {
                    self.daily_tracks.fetched_at_ms = Some(Self::now_millis());
                }
            }
            Err(err) => {
                self.daily_tracks.error = Some(err.to_string());
                self.daily_tracks.data.clear();
            }
        }
        self.daily_tracks.loading = false;
    }

    fn refresh_personal_fm(&mut self) {
        if let Some(fetched_at_ms) = self.personal_fm.fetched_at_ms
            && Self::cache_is_fresh(fetched_at_ms, HOME_PERSONAL_FM_TTL)
        {
            return;
        }
        self.personal_fm.loading = true;
        self.personal_fm.error = None;
        self.personal_fm.source = super::DataSource::User;
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            self.personal_fm.loading = false;
            self.personal_fm.error = Some("缺少鉴权凭据".to_string());
            return;
        };
        match library_actions::fetch_personal_fm_blocking(cookie.as_str()) {
            Ok(track) => {
                self.personal_fm.data = track;
                self.personal_fm.fetched_at_ms = Some(Self::now_millis());
            }
            Err(err) => {
                self.personal_fm.error = Some(err.to_string());
                self.personal_fm.data = None;
            }
        }
        self.personal_fm.loading = false;
    }

    pub(super) fn refresh_discover_playlists(&mut self) {
        self.discover_playlists.loading = true;
        self.discover_playlists.error = None;
        let is_user = self.has_user_token();
        let mut cache_key = if is_user {
            self.auth_user_id
                .map(|user_id| format!("discover.top_playlists.user.{user_id}"))
        } else {
            Some("discover.top_playlists.guest".to_string())
        };

        if let Some(key) = cache_key.as_deref()
            && let Some(entry) = self.read_cache(key, DISCOVER_PLAYLIST_TTL)
        {
            self.discover_playlists.data = entry.data;
            self.discover_playlists.fetched_at_ms = Some(entry.fetched_at_ms);
            self.discover_playlists.loading = false;
            self.discover_playlists.source = if is_user {
                super::DataSource::User
            } else {
                super::DataSource::Guest
            };
            return;
        }

        let cookie = if is_user {
            if let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) {
                cookie
            } else {
                cache_key = None;
                let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                    self.discover_playlists.loading = false;
                    self.discover_playlists.error = Some("缺少鉴权凭据".to_string());
                    return;
                };
                cookie
            }
        } else {
            let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
                self.discover_playlists.loading = false;
                self.discover_playlists.error = Some("缺少鉴权凭据".to_string());
                return;
            };
            cookie
        };

        match library_actions::fetch_top_playlists_blocking(60, 0, cookie.as_str()) {
            Ok(items) => {
                self.discover_playlists.data = items;
                if let Some(key) = cache_key.as_deref()
                    && let Some(fetched_at_ms) =
                        self.write_cache(key, &self.discover_playlists.data)
                {
                    self.discover_playlists.fetched_at_ms = Some(fetched_at_ms);
                }
            }
            Err(err) => {
                self.discover_playlists.data.clear();
                self.discover_playlists.error = Some(err.to_string());
            }
        }
        self.discover_playlists.loading = false;
    }

    fn build_playlist_page_from_remote(
        &mut self,
        playlist_id: i64,
    ) -> Result<playlist::PlaylistPage, String> {
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            return Err("缺少鉴权凭据".to_string());
        };
        let detail = library_actions::fetch_playlist_detail_blocking(playlist_id, cookie.as_str())
            .map_err(|err| err.to_string())?;
        Ok(playlist::PlaylistPage {
            id: detail.id,
            name: detail.name,
            creator_name: detail.creator_name,
            track_count: detail.track_count,
            tracks: detail
                .tracks
                .into_iter()
                .map(|track| playlist::PlaylistTrackRow {
                    id: track.id,
                    name: track.name,
                    artists: track.artists,
                    cover_url: track.cover_url,
                })
                .collect(),
        })
    }

    fn ensure_playlist_page_loaded(
        &mut self,
        playlist_id: i64,
    ) -> Result<playlist::PlaylistPage, String> {
        if let Some(page) = self.playlist_state.data.get(&playlist_id).cloned() {
            return Ok(page);
        }

        let cache_key = self
            .auth_user_id
            .map(|user_id| format!("playlist.detail.{playlist_id}.user.{user_id}"))
            .unwrap_or_else(|| format!("playlist.detail.{playlist_id}"));
        if let Some(entry) =
            self.read_cache::<playlist::PlaylistPage>(&cache_key, PLAYLIST_DETAIL_TTL)
        {
            self.playlist_state
                .data
                .insert(playlist_id, entry.data.clone());
            self.playlist_state.fetched_at_ms = Some(entry.fetched_at_ms);
            return Ok(entry.data);
        }

        let page = self.build_playlist_page_from_remote(playlist_id)?;
        self.playlist_state.data.insert(playlist_id, page.clone());
        if let Some(fetched_at_ms) = self.write_cache(&cache_key, &page) {
            self.playlist_state.fetched_at_ms = Some(fetched_at_ms);
        }
        Ok(page)
    }

    pub(super) fn open_playlist_from_library(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        Self::navigate_to(format!("/playlist/{playlist_id}"), cx);

        self.playlist_state.loading = true;
        self.playlist_state.error = None;
        self.playlist_state.source = super::DataSource::Guest;
        cx.notify();

        if let Err(err) = self.ensure_playlist_page_loaded(playlist_id) {
            self.playlist_state.error = Some(err);
        }

        self.playlist_state.loading = false;
        cx.notify();
    }

    pub(super) fn ensure_guest_session(&mut self) {
        if self.has_guest_token() {
            return;
        }
        if self.ensure_guest_token() {
            self.login_qr_status = Some("已获取游客凭据".to_string());
        }
    }

    pub(super) fn refresh_login_token(&mut self) {
        if self.auth_bundle.music_u.is_none() {
            Self::push_error(
                &mut self.search_error,
                "当前不是账号登录态，无法刷新登录令牌".to_string(),
            );
            return;
        }

        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            return;
        };
        match auth_actions::refresh_login_token_blocking(Some(cookie.as_str())) {
            Ok(response) => {
                self.merge_auth_cookies(&response.set_cookie);
                self.refresh_login_summary();
                self.login_qr_status = Some("登录令牌已刷新".to_string());
            }
            Err(err) => {
                Self::push_error(&mut self.search_error, format!("刷新登录令牌失败: {err}"));
            }
        }
    }

    pub(super) fn submit_search_from_query(&mut self, cx: &mut Context<Self>) {
        let query = self.app.read(cx).search_query.trim().to_string();
        if query.is_empty() {
            self.search_state.data.clear();
            self.search_state.error = None;
            self.search_state.loading = false;
            Self::navigate_to("/search", cx);
            return;
        }

        let sanitized = query.replace('/', " ");
        let path = format!("/search/{sanitized}");
        self.perform_search(query, cx);
        Self::navigate_to(path, cx);
    }

    pub(super) fn generate_login_qr(&mut self, cx: &mut Context<Self>) {
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            return;
        };
        let response = match auth_actions::fetch_login_qr_key_blocking(Some(cookie.as_str())) {
            Ok(response) => response,
            Err(err) => {
                Self::push_error(
                    &mut self.search_error,
                    format!("获取二维码 key 失败: {err}"),
                );
                return;
            }
        };

        let key = response
            .body
            .get("unikey")
            .and_then(|value| value.as_str())
            .or_else(|| {
                response.body["data"]
                    .get("unikey")
                    .and_then(|value| value.as_str())
            })
            .map(ToString::to_string);

        let Some(key) = key else {
            Self::push_error(&mut self.search_error, "二维码 key 为空".to_string());
            return;
        };

        let qr_url = format!("https://music.163.com/login?codekey={key}");
        let image_data = match QrCode::new(qr_url.as_bytes()) {
            Ok(code) => {
                let svg = code
                    .render::<svg::Color<'_>>()
                    .min_dimensions(280, 280)
                    .build();
                Some(Arc::new(Image::from_bytes(
                    ImageFormat::Svg,
                    svg.into_bytes(),
                )))
            }
            Err(err) => {
                Self::push_error(&mut self.search_error, format!("渲染二维码失败: {err}"));
                None
            }
        };

        self.login_qr_key = Some(key);
        self.login_qr_url = Some(qr_url);
        self.login_qr_image = image_data;
        self.login_qr_status = Some("801 等待扫码".to_string());
        self.login_qr_polling = true;
        self.login_qr_poll_started_at = Some(std::time::Instant::now());
        self.login_qr_last_polled_at = None;
        cx.notify();
    }

    pub(super) fn stop_login_qr_polling(&mut self, cx: &mut Context<Self>) {
        self.login_qr_polling = false;
        self.login_qr_status = Some("已停止轮询".to_string());
        cx.notify();
    }

    pub(crate) fn set_close_behavior(&mut self, value: CloseBehavior, cx: &mut Context<Self>) {
        if self.close_behavior == value {
            return;
        }
        self.close_behavior = value;
        self.persist_player_settings(cx);
        self.login_qr_status = Some(format!("关闭行为已切换为：{}", value.label()));
        cx.notify();
    }

    pub(super) fn tick_qr_poll(&mut self, now: std::time::Instant) -> bool {
        if !self.login_qr_polling {
            return false;
        }
        if let Some(started_at) = self.login_qr_poll_started_at
            && now.duration_since(started_at) >= QR_POLL_TIMEOUT
        {
            self.login_qr_polling = false;
            self.login_qr_status = Some("800 二维码过期".to_string());
            return true;
        }
        if let Some(last) = self.login_qr_last_polled_at
            && now.duration_since(last) < QR_POLL_INTERVAL
        {
            return false;
        }
        let Some(key) = self.login_qr_key.clone() else {
            self.login_qr_polling = false;
            self.login_qr_status = Some("二维码 key 丢失".to_string());
            return true;
        };
        self.login_qr_last_polled_at = Some(now);

        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            self.login_qr_polling = false;
            return true;
        };
        match auth_actions::check_login_qr_blocking(&key, Some(cookie.as_str())) {
            Ok(response) => {
                let code = response
                    .body
                    .get("code")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(500);
                match code {
                    800 => {
                        self.login_qr_polling = false;
                        self.login_qr_status = Some("800 二维码过期".to_string());
                    }
                    801 => {
                        self.login_qr_status = Some("801 等待扫码".to_string());
                    }
                    802 => {
                        self.login_qr_status = Some("802 待确认".to_string());
                    }
                    803 => {
                        self.login_qr_polling = false;
                        self.login_qr_status = Some("803 登录成功".to_string());
                        self.merge_auth_cookies(&response.set_cookie);
                        self.refresh_login_summary();
                    }
                    value => {
                        self.login_qr_status = Some(format!("{value} 登录状态未知"));
                    }
                }
            }
            Err(err) => {
                Self::push_error(&mut self.search_error, format!("二维码状态轮询失败: {err}"));
            }
        }
        true
    }

    fn prepare_track_source(
        &mut self,
        track_id: i64,
        queue_index: usize,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let cookie = self.ensure_auth_cookie(AuthLevel::Guest)?;
        let source_url =
            match player_actions::fetch_track_url_blocking(track_id, Some(cookie.as_str())) {
                Ok(url) => url,
                Err(err) => {
                    self.search_error = Some(format!("获取播放地址失败: {err}"));
                    cx.notify();
                    return None;
                }
            };

        self.player.update(cx, |player, _| {
            if let Some(item) = player.queue.get_mut(queue_index) {
                item.source_url = Some(source_url.clone());
            }
        });
        Some(source_url)
    }

    fn start_playback_at(
        &mut self,
        queue_index: usize,
        start_ms: u64,
        autoplay: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let snapshot = self.player.read(cx).clone();
        let Some(item) = snapshot.queue.get(queue_index).cloned() else {
            return false;
        };

        let Some(source_url) = self.prepare_track_source(item.id, queue_index, cx) else {
            self.persist_player_runtime(cx);
            return false;
        };

        if let Some(audio) = &self.audio_bridge
            && let Err(err) = audio.send(AudioCommand::Open {
                source: SourceSpec::network(source_url),
                start_ms,
                autoplay,
            })
        {
            self.search_error = Some(format!("播放失败: {err}"));
            cx.notify();
            return false;
        }

        self.player.update(cx, |player, _| {
            player.current_index = Some(queue_index);
            player.is_playing = autoplay;
        });
        self.persist_player_runtime(cx);
        cx.notify();
        true
    }

    fn refresh_current_track_url_and_resume(&mut self, cx: &mut Context<Self>) -> bool {
        let player_snapshot = self.player.read(cx).clone();
        let Some(current_index) = player_snapshot.current_index else {
            return false;
        };
        let Some(current_item) = player_snapshot.queue.get(current_index).cloned() else {
            return false;
        };

        let Some(url) = self.prepare_track_source(current_item.id, current_index, cx) else {
            self.search_error = Some("刷新播放地址失败".to_string());
            return false;
        };

        if let Some(audio) = &self.audio_bridge {
            let position = audio.service().snapshot().position_ms;
            if let Err(err) = audio.send(AudioCommand::Open {
                source: SourceSpec::network(url),
                start_ms: position,
                autoplay: true,
            }) {
                self.search_error = Some(format!("刷新播放失败: {err}"));
                cx.notify();
                return false;
            }
        }

        self.player.update(cx, |player, _| {
            player.is_playing = true;
        });
        self.persist_player_runtime(cx);
        cx.notify();
        true
    }

    pub(super) fn perform_search(&mut self, query: String, cx: &mut Context<Self>) {
        let query = query.trim().to_string();
        self.search_state.loading = true;
        self.search_state.error = None;
        self.search_state.source = super::DataSource::Guest;
        cx.notify();

        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            self.search_state.data.clear();
            self.search_state.loading = false;
            self.search_state.error = Some("缺少鉴权凭据".to_string());
            cx.notify();
            return;
        };
        let result = search_actions::search_song_blocking(&query, Some(cookie.as_str()));
        match result {
            Ok(items) => {
                self.search_state.data = items
                    .into_iter()
                    .map(|item| search::SearchSong {
                        id: item.id,
                        name: item.name,
                        artists: item.artists,
                    })
                    .collect();
                self.search_state.loading = false;
                self.search_state.error = None;
            }
            Err(err) => {
                self.search_state.data.clear();
                self.search_state.loading = false;
                self.search_state.error = Some(err.to_string());
            }
        }
        cx.notify();
    }

    pub(super) fn play_current(&mut self, cx: &mut Context<Self>) {
        let player = self.player.read(cx).clone();
        let Some(current_index) = player.current_index else {
            return;
        };
        self.start_playback_at(current_index, player.position_ms, true, cx);
    }

    pub(super) fn enqueue_song_from_route(
        &mut self,
        song: search::SearchSong,
        cx: &mut Context<Self>,
    ) {
        if let Some(existing_index) = queue_actions::index_of(self.player.read(cx), song.id) {
            self.start_playback_at(existing_index, 0, true, cx);
            return;
        }

        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            return;
        };
        let metadata =
            player_actions::fetch_track_metadata_blocking(song.id, Some(cookie.as_str())).ok();
        let artists = metadata
            .as_ref()
            .map(|meta| meta.artists.clone())
            .unwrap_or_else(|| song.artists.clone());
        let cover_url = metadata.and_then(|meta| meta.cover_url);

        let mut inserted_index = None;
        self.player.update(cx, |player, _| {
            player.enqueue(QueueItem {
                id: song.id,
                name: song.name.clone(),
                artist: artists,
                cover_url,
                source_url: None,
            });
            inserted_index = player.queue.len().checked_sub(1);
            player.current_index = inserted_index;
        });

        if let Some(index) = inserted_index {
            self.start_playback_at(index, 0, true, cx);
        } else {
            self.persist_player_runtime(cx);
        }
    }

    pub(super) fn enqueue_song_without_play_from_route(
        &mut self,
        song: search::SearchSong,
        cx: &mut Context<Self>,
    ) {
        if let Some(existing_index) = queue_actions::index_of(self.player.read(cx), song.id) {
            self.start_playback_at(existing_index, 0, true, cx);
            return;
        }

        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            return;
        };
        let metadata =
            player_actions::fetch_track_metadata_blocking(song.id, Some(cookie.as_str())).ok();
        let artists = metadata
            .as_ref()
            .map(|meta| meta.artists.clone())
            .unwrap_or_else(|| song.artists.clone());
        let cover_url = metadata.and_then(|meta| meta.cover_url);

        self.player.update(cx, |player, _| {
            player.enqueue(QueueItem {
                id: song.id,
                name: song.name,
                artist: artists,
                cover_url,
                source_url: None,
            });
        });
        self.persist_player_runtime(cx);
        cx.notify();
    }

    pub(super) fn replace_queue_from_playlist(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        let page = match self.ensure_playlist_page_loaded(playlist_id) {
            Ok(page) => page,
            Err(err) => {
                Self::push_error(&mut self.search_error, format!("替换队列失败: {err}"));
                return;
            }
        };
        if page.tracks.is_empty() {
            Self::push_error(&mut self.search_error, "歌单为空，无法替换队列".to_string());
            return;
        }

        self.player.update(cx, |player, _| {
            player.set_queue(
                page.tracks
                    .iter()
                    .map(|track| QueueItem {
                        id: track.id,
                        name: track.name.clone(),
                        artist: track.artists.clone(),
                        cover_url: track.cover_url.clone(),
                        source_url: None,
                    })
                    .collect(),
            );
            player.current_index = Some(0);
            player.position_ms = 0;
            player.duration_ms = 0;
        });

        if !self.start_playback_at(0, 0, true, cx) {
            self.persist_player_runtime(cx);
            self.persist_player_progress(cx);
            cx.notify();
        }
    }

    pub(super) fn replace_queue_from_daily_tracks(
        &mut self,
        track_id: Option<i64>,
        cx: &mut Context<Self>,
    ) {
        if !self.has_user_token() {
            Self::push_error(&mut self.search_error, "每日推荐需要账号登录".to_string());
            return;
        }

        if self.daily_tracks.data.is_empty() {
            self.refresh_daily_tracks();
        }
        if self.daily_tracks.data.is_empty() {
            Self::push_error(
                &mut self.search_error,
                "每日推荐为空，无法替换队列".to_string(),
            );
            return;
        }

        let mut start_index = 0usize;
        if let Some(track_id) = track_id
            && let Some(index) = self
                .daily_tracks
                .data
                .iter()
                .position(|track| track.id == track_id)
        {
            start_index = index;
        }

        let queue = self
            .daily_tracks
            .data
            .iter()
            .map(|track| QueueItem {
                id: track.id,
                name: track.name.clone(),
                artist: track.artists.clone(),
                cover_url: track.cover_url.clone(),
                source_url: None,
            })
            .collect::<Vec<_>>();

        self.player.update(cx, |player, _| {
            player.set_queue(queue);
            player.current_index = Some(start_index);
            player.position_ms = 0;
            player.duration_ms = 0;
        });

        if !self.start_playback_at(start_index, 0, true, cx) {
            self.persist_player_runtime(cx);
            self.persist_player_progress(cx);
            cx.notify();
        }
    }

    pub(super) fn play_queue_item_from_route(&mut self, track_id: i64, cx: &mut Context<Self>) {
        let Some(index) = queue_actions::index_of(self.player.read(cx), track_id) else {
            return;
        };
        self.start_playback_at(index, 0, true, cx);
    }

    pub(super) fn remove_queue_item_from_route(&mut self, track_id: i64, cx: &mut Context<Self>) {
        self.player.update(cx, |player, _| {
            queue_actions::remove_by_id(player, track_id);
        });
        self.persist_player_runtime(cx);
    }

    pub(super) fn clear_queue_from_route(&mut self, cx: &mut Context<Self>) {
        self.player.update(cx, |player, _| {
            queue_actions::clear(player);
            player.is_playing = false;
            player.position_ms = 0;
            player.duration_ms = 0;
        });
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::Stop);
        }
        self.persist_player_runtime(cx);
        self.persist_player_progress(cx);
    }

    pub(super) fn play_previous(&mut self, cx: &mut Context<Self>) {
        let mut target = None;
        self.player.update(cx, |player, _| {
            target = player.prev_index();
            player.position_ms = 0;
            player.duration_ms = 0;
        });
        if let Some(index) = target {
            self.start_playback_at(index, 0, true, cx);
        }
    }

    pub(super) fn play_next(&mut self, cx: &mut Context<Self>) {
        let mut target = None;
        self.player.update(cx, |player, _| {
            target = player.next_index();
            player.position_ms = 0;
            player.duration_ms = 0;
        });
        if let Some(index) = target {
            self.start_playback_at(index, 0, true, cx);
        }
    }

    pub(super) fn cycle_play_mode(&mut self, cx: &mut Context<Self>) {
        self.player.update(cx, |player, _| player.cycle_mode());
        self.persist_player_settings(cx);
        self.persist_player_runtime(cx);
        cx.notify();
    }

    pub(super) fn set_volume_absolute(&mut self, volume: f32, cx: &mut Context<Self>) {
        let volume = volume.clamp(0.0, 1.0);
        self.player
            .update(cx, |player, _| player.set_volume(volume));
        self.player_volume_slider.update(cx, |slider, _| {
            if !slider.is_dragging() {
                slider.set_value_silent(volume);
            }
        });
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::SetVolume(volume));
        }
        self.persist_player_settings(cx);
        cx.notify();
    }

    pub(super) fn preview_seek_ratio(&mut self, ratio: f32, cx: &mut Context<Self>) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.player.update(cx, |player, _| {
            let duration = player.duration_ms.max(1);
            player.position_ms = ((duration as f32) * ratio) as u64;
        });
        cx.notify();
    }

    pub(super) fn commit_seek_ratio(&mut self, ratio: f32, cx: &mut Context<Self>) {
        let ratio = ratio.clamp(0.0, 1.0);
        let duration_ms = self.player.read(cx).duration_ms.max(1);
        let target_ms = ((duration_ms as f32) * ratio) as u64;
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::Seek(SeekTarget::ms(target_ms)));
        }
        self.player
            .update(cx, |player, _| player.position_ms = target_ms);
        self.persist_player_progress(cx);
        cx.notify();
    }

    pub(super) fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        let is_playing = self.player.read(cx).is_playing;
        if is_playing {
            if let Some(audio) = &self.audio_bridge {
                let _ = audio.send(AudioCommand::Pause);
            }
            self.player
                .update(cx, |player, _| player.is_playing = false);
            self.persist_player_runtime(cx);
            cx.notify();
            return;
        }

        if let Some(audio) = &self.audio_bridge {
            let snapshot = audio.service().snapshot();
            if snapshot.source.is_some() && audio.send(AudioCommand::Play).is_ok() {
                self.player.update(cx, |player, _| player.is_playing = true);
                self.persist_player_runtime(cx);
                cx.notify();
                return;
            }
        }

        self.play_current(cx);
    }

    pub(super) fn sync_audio_bridge(&mut self, cx: &mut Context<Self>) {
        let mut ended = false;
        let mut forbidden = false;
        let mut last_error: Option<String> = None;
        if let Some(bridge) = self.audio_bridge.as_mut() {
            self.player.update(cx, |player, _| {
                for event in bridge.drain(player) {
                    match event {
                        ame_audio::AudioEvent::TrackEnded => ended = true,
                        ame_audio::AudioEvent::Error(err) => {
                            if matches!(err, AudioError::HttpStatus { code: 403, .. }) {
                                forbidden = true;
                            }
                        }
                        _ => {}
                    }
                }
            });
            last_error = bridge.last_error.take();
        }

        if forbidden && self.refresh_current_track_url_and_resume(cx) {
            return;
        }

        if let Some(err) = last_error {
            self.search_error = Some(err);
        }

        let player_snapshot = self.player.read(cx).clone();
        self.player_volume_slider.update(cx, |slider, _| {
            if !slider.is_dragging() {
                slider.set_value_silent(player_snapshot.volume);
            }
        });
        self.player_progress_slider.update(cx, |slider, _| {
            if !slider.is_dragging() {
                slider.set_value_silent(player_snapshot.progress_ratio());
            }
        });

        if ended {
            let mut target = None;
            self.player.update(cx, |player, _| {
                target = player.next_index();
                player.position_ms = 0;
                player.duration_ms = 0;
            });
            if let Some(index) = target {
                self.start_playback_at(index, 0, true, cx);
            }
        }
    }

    pub(crate) fn request_window_close(
        &mut self,
        window: &mut nekowg::Window,
        cx: &mut Context<Self>,
    ) {
        match self.close_behavior {
            CloseBehavior::HideToTray => {
                window.hide();
            }
            CloseBehavior::Exit => {
                self.prepare_app_exit(cx);
                cx.quit();
            }
            CloseBehavior::Ask => {
                let window_handle = window.window_handle();
                let answer = window.prompt(
                    PromptLevel::Info,
                    "确定要关闭吗？",
                    Some("以下选择会作为默认行为，可以在设置中修改"),
                    &[
                        PromptButton::new("隐藏到托盘"),
                        PromptButton::ok("退出应用"),
                        PromptButton::cancel("取消"),
                    ],
                    cx,
                );
                let root = cx.entity();
                cx.spawn(async move |_, cx| {
                    let Ok(choice) = answer.await else {
                        return;
                    };
                    root.update(cx, |this, cx| match choice {
                        0 => {
                            this.set_close_behavior(CloseBehavior::HideToTray, cx);
                            let _ = window_handle.update(cx, |_, window, _cx| {
                                window.hide();
                            });
                        }
                        1 => {
                            this.set_close_behavior(CloseBehavior::Exit, cx);
                            this.prepare_app_exit(cx);
                            cx.quit();
                        }
                        _ => {}
                    });
                })
                .detach();
            }
        }
    }

    pub(crate) fn prepare_app_exit(&mut self, cx: &mut Context<Self>) {
        self.login_qr_polling = false;
        let _ = self
            .kernel_runtime
            .command_sender()
            .send(AppCommand::Shutdown);
        self.persist_player_settings(cx);
        self.persist_player_runtime(cx);
        self.persist_player_progress(cx);
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::Stop);
        }
    }

    pub(crate) fn tray_toggle_playback(&mut self, cx: &mut Context<Self>) {
        self.queue_kernel_command(AppCommand::TogglePlay);
        cx.notify();
    }

    pub(crate) fn tray_next(&mut self, cx: &mut Context<Self>) {
        self.queue_kernel_command(AppCommand::NextTrack);
        cx.notify();
    }

    pub(super) fn navigate_to(path: impl Into<SharedString>, cx: &mut Context<Self>) {
        navigate(cx, path.into());
        cx.notify();
    }
}

fn song_input_to_search_song(song: SongInput) -> search::SearchSong {
    search::SearchSong {
        id: song.id,
        name: song.name,
        artists: song.artists,
    }
}
