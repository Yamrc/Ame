use gpui::{Context, SharedString};
use gpui_router::use_navigate;

use ame_audio::{AudioCommand, AudioError, SeekTarget, SourceSpec};
use gpui::{Image, ImageFormat};
use qrcode::{QrCode, render::svg};
use std::sync::Arc;

use crate::action::{auth_actions, library_actions, player_actions, queue_actions, search_actions};
use crate::entity::app::CloseBehavior;
use crate::entity::player::QueueItem;
use crate::kernel::{AppCommand, AppEvent, KernelCommandSender, SongInput};
use crate::view::{library, playlist, search};

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

    pub(super) fn refresh_login_summary(&mut self) {
        if self.auth_bundle.music_u.is_none() {
            self.auth_account_summary = None;
            self.auth_user_id = None;
            self.library_playlists.clear();
            self.playlist_pages.clear();
            return;
        }
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            self.auth_account_summary = None;
            self.auth_user_id = None;
            return;
        };
        match auth_actions::fetch_login_status_blocking(Some(cookie.as_str())) {
            Ok(body) => {
                self.auth_account_summary = auth_actions::login_summary_text(&body);
                self.auth_user_id = body["data"]["account"]["id"]
                    .as_i64()
                    .or_else(|| body["data"]["profile"]["userId"].as_i64());
                self.refresh_library_playlists();
                self.refresh_home_playlists();
                self.refresh_discover_playlists();
            }
            Err(err) => {
                self.auth_account_summary = None;
                self.auth_user_id = None;
                Self::push_error(&mut self.search_error, format!("读取登录状态失败: {err}"));
            }
        }
    }

    pub(super) fn refresh_library_playlists(&mut self) {
        let Some(user_id) = self.auth_user_id else {
            self.library_playlists.clear();
            self.library_error = None;
            self.library_loading = false;
            return;
        };

        self.library_loading = true;
        self.library_error = None;
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::User) else {
            self.library_loading = false;
            self.library_error = Some("缺少鉴权凭据".to_string());
            return;
        };
        match library_actions::fetch_user_playlists_blocking(user_id, cookie.as_str()) {
            Ok(items) => {
                self.library_playlists = items
                    .into_iter()
                    .map(|item| library::LibraryPlaylistCard {
                        id: item.id,
                        name: item.name,
                        track_count: item.track_count,
                        creator_name: item.creator_name,
                        cover_url: item.cover_url,
                    })
                    .collect();
            }
            Err(err) => {
                self.library_playlists.clear();
                self.library_error = Some(err.to_string());
            }
        }
        self.library_loading = false;
    }

    pub(super) fn refresh_home_playlists(&mut self) {
        self.home_loading = true;
        self.home_error = None;
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            self.home_loading = false;
            self.home_error = Some("缺少鉴权凭据".to_string());
            return;
        };
        match library_actions::fetch_top_playlists_blocking(20, 0, cookie.as_str()) {
            Ok(items) => {
                self.home_playlists = items
                    .into_iter()
                    .map(|item| library::LibraryPlaylistCard {
                        id: item.id,
                        name: item.name,
                        track_count: item.track_count,
                        creator_name: item.creator_name,
                        cover_url: item.cover_url,
                    })
                    .collect();
            }
            Err(err) => {
                self.home_playlists.clear();
                self.home_error = Some(err.to_string());
            }
        }
        self.home_loading = false;
    }

    pub(super) fn refresh_discover_playlists(&mut self) {
        self.discover_loading = true;
        self.discover_error = None;
        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            self.discover_loading = false;
            self.discover_error = Some("缺少鉴权凭据".to_string());
            return;
        };
        match library_actions::fetch_top_playlists_blocking(60, 0, cookie.as_str()) {
            Ok(items) => {
                self.discover_playlists = items
                    .into_iter()
                    .map(|item| library::LibraryPlaylistCard {
                        id: item.id,
                        name: item.name,
                        track_count: item.track_count,
                        creator_name: item.creator_name,
                        cover_url: item.cover_url,
                    })
                    .collect();
            }
            Err(err) => {
                self.discover_playlists.clear();
                self.discover_error = Some(err.to_string());
            }
        }
        self.discover_loading = false;
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
                })
                .collect(),
        })
    }

    fn ensure_playlist_page_loaded(
        &mut self,
        playlist_id: i64,
    ) -> Result<playlist::PlaylistPage, String> {
        if let Some(page) = self.playlist_pages.get(&playlist_id).cloned() {
            return Ok(page);
        }

        let page = self.build_playlist_page_from_remote(playlist_id)?;
        self.playlist_pages.insert(playlist_id, page.clone());
        Ok(page)
    }

    pub(super) fn open_playlist_from_library(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        Self::navigate_to(format!("/playlist/{playlist_id}"), cx);

        self.playlist_loading = true;
        self.playlist_error = None;
        cx.notify();

        if let Err(err) = self.ensure_playlist_page_loaded(playlist_id) {
            self.playlist_error = Some(err);
        }

        self.playlist_loading = false;
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
            self.search_results.clear();
            self.search_error = None;
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
        self.search_loading = true;
        self.search_error = None;
        cx.notify();

        let Some(cookie) = self.ensure_auth_cookie(AuthLevel::Guest) else {
            self.search_results.clear();
            self.search_loading = false;
            self.search_error = Some("缺少鉴权凭据".to_string());
            cx.notify();
            return;
        };
        let result = search_actions::search_song_blocking(&query, Some(cookie.as_str()));
        match result {
            Ok(items) => {
                self.search_results = items
                    .into_iter()
                    .map(|item| search::SearchSong {
                        id: item.id,
                        name: item.name,
                        artists: item.artists,
                    })
                    .collect();
                self.search_loading = false;
                self.search_error = None;
            }
            Err(err) => {
                self.search_results.clear();
                self.search_loading = false;
                self.search_error = Some(err.to_string());
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
                        cover_url: None,
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
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        // TODO: 恢复按 close_behavior 分支处理（HideToTray / Ask / Exit）。
        // 懒狗了，先不搞后台，反正MVP能用就行
        self.prepare_app_exit(cx);
        cx.quit();
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
        {
            let mut navigate = use_navigate(cx);
            navigate(path.into());
        }
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
