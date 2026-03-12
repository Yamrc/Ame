use crate::router::{Route, Routes, use_params};
use nekowg::{
    AnyElement, App, Context, Entity, ListSizingBehavior, MouseButton, ScrollHandle, prelude::*, px,
};
use std::{collections::HashMap, sync::Arc};

use crate::action::library_actions;
use crate::component::{button, virtual_list};
use crate::entity::app::CloseBehavior;
use crate::entity::player::PlayerEntity;
use crate::kernel::{AppCommand, SongInput};
use crate::view::{daily_tracks, discover, home, library, login, next, playlist, search, settings};

use super::{DataState, RootView};

pub(super) struct RoutesModel {
    pub home_recommend_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    pub home_recommend_artists: DataState<Vec<library_actions::ArtistItem>>,
    pub home_new_albums: DataState<Vec<library_actions::AlbumItem>>,
    pub home_toplists: DataState<Vec<library_actions::ToplistItem>>,
    pub daily_tracks: DataState<Vec<library_actions::DailyTrackItem>>,
    pub personal_fm: DataState<Option<library_actions::FmTrackItem>>,
    pub is_user_logged_in: bool,
    pub discover_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    pub search_state: DataState<Vec<search::SearchSong>>,
    pub library_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    pub library_liked_tracks: DataState<Vec<library_actions::PlaylistTrackItem>>,
    pub library_liked_lyric_lines: Vec<String>,
    pub library_tab: library::LibraryTab,
    pub playlist_state: DataState<HashMap<i64, playlist::PlaylistPage>>,
    pub page_scroll_handle: ScrollHandle,
    pub auth_account_summary: Option<String>,
    pub auth_user_name: Option<String>,
    pub auth_user_avatar: Option<String>,
    pub login_model: login::LoginViewModel,
    pub close_behavior_label: String,
}

impl RootView {
    fn build_home_featured_rows(
        daily_tracks: &[library_actions::DailyTrackItem],
        fm_track: Option<&library_actions::FmTrackItem>,
        root_entity: &Entity<RootView>,
        is_user_logged_in: bool,
    ) -> Vec<AnyElement> {
        let mut rows = Vec::with_capacity(2);

        let daily_first_id = daily_tracks.first().map(|track| track.id);
        let daily_cover = daily_tracks
            .first()
            .and_then(|track| track.cover_url.clone());
        let daily_card = home::HomePlaylistCard {
            id: 0,
            name: "每日推荐".to_string(),
            subtitle: "根据你的口味更新".to_string(),
            cover_url: daily_cover,
        };
        let daily_root = root_entity.clone();
        rows.push(home::daily_featured_card(
            daily_card,
            {
                let daily_root = daily_root.clone();
                move |cx| {
                    daily_root.update(cx, |this, _| {
                        if is_user_logged_in {
                            this.queue_kernel_command(AppCommand::Navigate(
                                "/daily/songs".to_string(),
                            ));
                        } else {
                            this.queue_kernel_command(AppCommand::Navigate("/login".to_string()));
                        }
                    });
                }
            },
            move |cx| {
                daily_root.update(cx, |this, _| {
                    if is_user_logged_in {
                        this.queue_kernel_command(AppCommand::ReplaceQueueFromDailyTracks(
                            daily_first_id,
                        ));
                    } else {
                        this.queue_kernel_command(AppCommand::Navigate("/login".to_string()));
                    }
                });
            },
        ));

        let fm_card = fm_track
            .map(|track| home::HomePlaylistCard {
                id: track.id,
                name: track.name.clone(),
                subtitle: track.artists.clone(),
                cover_url: track.cover_url.clone(),
            })
            .unwrap_or(home::HomePlaylistCard {
                id: 0,
                name: "私人 FM".to_string(),
                subtitle: "连续播放你可能喜欢的音乐".to_string(),
                cover_url: None,
            });
        let fm_root = root_entity.clone();
        let fm_track = fm_track.cloned();
        rows.push(home::fm_featured_card(fm_card, move |cx| {
            fm_root.update(cx, |this, _| {
                if is_user_logged_in {
                    if let Some(track) = fm_track.clone() {
                        this.queue_kernel_command(AppCommand::EnqueueSongAndPlay(SongInput {
                            id: track.id,
                            name: track.name,
                            artists: track.artists,
                        }));
                    } else {
                        this.queue_kernel_command(AppCommand::Navigate("/library".to_string()));
                    }
                } else {
                    this.queue_kernel_command(AppCommand::Navigate("/login".to_string()));
                }
            });
        }));

        rows
    }
    fn build_home_playlist_rows(
        playlists: &[library_actions::LibraryPlaylistItem],
        root_entity: &Entity<RootView>,
    ) -> Vec<AnyElement> {
        playlists
            .iter()
            .take(10)
            .cloned()
            .map(|item| {
                let playlist_id = item.id;
                let root_entity = root_entity.clone();
                home::playlist_card(
                    home::HomePlaylistCard {
                        id: item.id,
                        name: item.name,
                        subtitle: item.creator_name,
                        cover_url: item.cover_url,
                    },
                    move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::OpenLibraryPlaylist(playlist_id))
                        });
                    },
                )
            })
            .collect()
    }

    fn build_home_artist_rows(artists: &[library_actions::ArtistItem]) -> Vec<AnyElement> {
        artists
            .iter()
            .take(6)
            .cloned()
            .map(|artist| home::artist_card(artist.name, artist.cover_url, move |_cx| {}))
            .collect()
    }

    fn build_home_album_rows(albums: &[library_actions::AlbumItem]) -> Vec<AnyElement> {
        albums
            .iter()
            .take(10)
            .cloned()
            .map(|album| {
                home::playlist_card(
                    home::HomePlaylistCard {
                        id: album.id,
                        name: album.name,
                        subtitle: album.artist_name,
                        cover_url: album.cover_url,
                    },
                    move |_cx| {},
                )
            })
            .collect()
    }

    fn build_home_toplist_rows(toplists: &[library_actions::ToplistItem]) -> Vec<AnyElement> {
        toplists
            .iter()
            .take(10)
            .cloned()
            .map(|list| {
                home::playlist_card(
                    home::HomePlaylistCard {
                        id: list.id,
                        name: list.name,
                        subtitle: list.update_frequency,
                        cover_url: list.cover_url,
                    },
                    move |_cx| {},
                )
            })
            .collect()
    }

    fn build_search_rows(
        results: &[search::SearchSong],
        root_entity: &Entity<RootView>,
    ) -> Vec<AnyElement> {
        results
            .iter()
            .cloned()
            .map(|song| {
                let song_for_click = song.clone();
                let root_entity = root_entity.clone();
                search::render_row(song, move |cx| {
                    root_entity.update(cx, |this, _| {
                        this.queue_kernel_command(AppCommand::EnqueueSongAndPlay(SongInput {
                            id: song_for_click.id,
                            name: song_for_click.name.clone(),
                            artists: song_for_click.artists.clone(),
                        }))
                    });
                })
            })
            .collect()
    }

    fn render_search_route(
        keyword: &str,
        loading: bool,
        error: Option<&str>,
        results: &[search::SearchSong],
        root_entity: &Entity<RootView>,
    ) -> AnyElement {
        let rows = Self::build_search_rows(results, root_entity);
        search::render(keyword, loading, error, rows)
    }

    fn build_library_rows(
        playlists: &[library_actions::LibraryPlaylistItem],
        root_entity: &Entity<RootView>,
    ) -> Vec<AnyElement> {
        playlists
            .iter()
            .cloned()
            .map(|item| {
                let playlist_id = item.id;
                let root_entity = root_entity.clone();
                library::playlist_row(
                    library::LibraryPlaylistCard {
                        id: item.id,
                        name: item.name,
                        track_count: item.track_count,
                        creator_name: item.creator_name,
                        cover_url: item.cover_url,
                    },
                    move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::OpenLibraryPlaylist(playlist_id))
                        });
                    },
                )
            })
            .collect()
    }

    fn build_discover_rows(
        playlists: &[library_actions::LibraryPlaylistItem],
        root_entity: &Entity<RootView>,
    ) -> Vec<AnyElement> {
        playlists
            .iter()
            .take(12)
            .cloned()
            .map(|item| {
                let playlist_id = item.id;
                let root_entity = root_entity.clone();
                discover::playlist_card(
                    discover::DiscoverPlaylistCard {
                        id: item.id,
                        name: item.name,
                        track_count: item.track_count,
                        creator_name: item.creator_name,
                        cover_url: item.cover_url,
                    },
                    move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::OpenLibraryPlaylist(playlist_id))
                        });
                    },
                )
            })
            .collect()
    }

    pub(super) fn render_routes(
        &self,
        cx: &mut Context<Self>,
        root_entity: Entity<RootView>,
        player_entity: Entity<PlayerEntity>,
        model: RoutesModel,
    ) -> AnyElement {
        let RoutesModel {
            home_recommend_playlists: home_recommend_playlists_state,
            home_recommend_artists: home_recommend_artists_state,
            home_new_albums: home_new_albums_state,
            home_toplists: home_toplists_state,
            daily_tracks: daily_tracks_state,
            personal_fm: personal_fm_state,
            is_user_logged_in,
            discover_playlists: discover_playlists_state,
            search_state,
            library_playlists: library_playlists_state,
            library_liked_tracks: library_liked_tracks_state,
            library_liked_lyric_lines,
            library_tab,
            playlist_state: playlist_state_state,
            page_scroll_handle,
            auth_account_summary,
            auth_user_name,
            auth_user_avatar,
            login_model,
            close_behavior_label,
        } = model;

        let home_loading = home_recommend_playlists_state.loading
            || home_recommend_artists_state.loading
            || home_new_albums_state.loading
            || home_toplists_state.loading
            || daily_tracks_state.loading
            || personal_fm_state.loading;
        let home_error = home_recommend_playlists_state
            .error
            .as_deref()
            .or(home_recommend_artists_state.error.as_deref())
            .or(home_new_albums_state.error.as_deref())
            .or(home_toplists_state.error.as_deref())
            .or(daily_tracks_state.error.as_deref())
            .or(personal_fm_state.error.as_deref());

        Routes::new()
            .basename("/")
            .child(Route::new().index().element({
                let root_entity = root_entity.clone();
                let home_playlists = home_recommend_playlists_state.data.clone();
                let artist_rows = home_recommend_artists_state.data.clone();
                let album_rows = home_new_albums_state.data.clone();
                let toplist_rows = home_toplists_state.data.clone();
                let daily_tracks = daily_tracks_state.data.clone();
                let personal_fm = personal_fm_state.data.clone();
                let home_error = home_error.map(str::to_string);
                move |_, _| {
                    home::render(
                        home_loading,
                        home_error.as_deref(),
                        Self::build_home_featured_rows(
                            &daily_tracks,
                            personal_fm.as_ref(),
                            &root_entity,
                            is_user_logged_in,
                        ),
                        Self::build_home_playlist_rows(&home_playlists, &root_entity),
                        Self::build_home_artist_rows(&artist_rows),
                        Self::build_home_album_rows(&album_rows),
                        Self::build_home_toplist_rows(&toplist_rows),
                    )
                }
            }))
            .child(Route::new().path("explore").element({
                let root_entity = root_entity.clone();
                let discover_playlists = discover_playlists_state.data.clone();
                let discover_loading = discover_playlists_state.loading;
                let discover_error = discover_playlists_state.error.clone();
                move |_, _| {
                    discover::render(
                        discover_loading,
                        discover_error.as_deref(),
                        Self::build_discover_rows(&discover_playlists, &root_entity),
                    )
                }
            }))
            .child(Route::new().path("library").element({
                let root_entity = root_entity.clone();
                let library_playlists = library_playlists_state.data.clone();
                let library_liked_tracks = library_liked_tracks_state.data.clone();
                let library_loading = library_playlists_state.loading;
                let library_error = library_playlists_state.error.clone();
                let auth_account_summary = auth_account_summary.clone();
                let auth_user_name = auth_user_name.clone();
                let auth_user_avatar = auth_user_avatar.clone();
                move |_, _| {
                    let liked_playlist = library_playlists
                        .iter()
                        .find(|item| item.special_type == 5)
                        .cloned();
                    let created_items = library_playlists
                        .iter()
                        .filter(|item| !item.subscribed && item.special_type != 5)
                        .cloned()
                        .collect::<Vec<_>>();
                    let collected_items = library_playlists
                        .iter()
                        .filter(|item| item.subscribed)
                        .cloned()
                        .collect::<Vec<_>>();
                    let created_rows = Self::build_library_rows(&created_items, &root_entity);
                    let collected_rows = Self::build_library_rows(&collected_items, &root_entity);
                    let preview_count = library_liked_tracks.len().min(12);
                    let preview_rows = preview_count.div_ceil(3).max(2);
                    let preview_height = preview_rows as f32 * 52.0
                        + (preview_rows.saturating_sub(1) as f32) * 8.0;
                    let preview_min_height = px(preview_height);
                    let title = auth_user_name
                        .as_deref()
                        .filter(|name| !name.trim().is_empty())
                        .map(|name| format!("{name} 的音乐库"))
                        .or_else(|| {
                            auth_account_summary
                                .as_deref()
                                .filter(|summary| !summary.trim().is_empty())
                                .map(|summary| format!("{summary} 的音乐库"))
                        })
                        .unwrap_or_else(|| "我的音乐库".to_string());
                    let root_for_liked = root_entity.clone();
                    let liked_lyrics = library_liked_lyric_lines.clone();
                    let liked_card = liked_playlist.clone().map(|item| {
                        let playlist_id = item.id;
                        let root_entity = root_for_liked.clone();
                        let root_for_open = root_entity.clone();
                        let root_for_play = root_entity.clone();
                        library::liked_card(
                            library::LibraryPlaylistCard {
                                id: item.id,
                                name: item.name,
                                track_count: item.track_count,
                                creator_name: item.creator_name,
                                cover_url: item.cover_url,
                            },
                            &liked_lyrics,
                            preview_min_height,
                            move |cx| {
                                root_for_open.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::OpenLibraryPlaylist(
                                        playlist_id,
                                    ))
                                });
                            },
                            move |cx| {
                                root_for_play.update(cx, |this, _| {
                                    this.queue_kernel_command(
                                        AppCommand::ReplaceQueueFromPlaylist(playlist_id),
                                    )
                                });
                            },
                        )
                    });
                    let preview_play = {
                        let root_entity = root_entity.clone();
                        Arc::new(move |track: library_actions::PlaylistTrackItem, cx: &mut App| {
                            root_entity.update(cx, |this, _| {
                                this.queue_kernel_command(AppCommand::EnqueueSongAndPlay(
                                    SongInput {
                                        id: track.id,
                                        name: track.name,
                                        artists: track.artists,
                                    },
                                ))
                            });
                        })
                    };
                    let model = library::LibraryViewModel {
                        title: title.clone().into(),
                        user_avatar: auth_user_avatar.clone(),
                        loading: library_loading,
                        error: library_error.clone().map(Into::into),
                        liked_card,
                        liked_tracks: library_liked_tracks.clone(),
                        preview_min_height,
                        active_tab: library_tab,
                        created_rows,
                        collected_rows,
                        followed_rows: Vec::new(),
                    };
                    let actions = library::LibraryActions {
                        on_tab_created: {
                            let root_entity = root_entity.clone();
                            Arc::new(move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.library_tab = library::LibraryTab::Created;
                                });
                            })
                        },
                        on_tab_collected: {
                            let root_entity = root_entity.clone();
                            Arc::new(move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.library_tab = library::LibraryTab::Collected;
                                });
                            })
                        },
                        on_tab_followed: {
                            let root_entity = root_entity.clone();
                            Arc::new(move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.library_tab = library::LibraryTab::Followed;
                                });
                            })
                        },
                        on_preview_play: preview_play,
                    };
                    library::render(model, actions)
                }
            }))
            .child(Route::new().path("search").element({
                let results = search_state.data.clone();
                let search_loading = search_state.loading;
                let error = search_state.error.clone();
                let root_entity = root_entity.clone();
                move |_, _| {
                    Self::render_search_route(
                        "",
                        search_loading,
                        error.as_deref(),
                        &results,
                        &root_entity,
                    )
                }
            }))
            .child(Route::new().path("search/{keywords}").element({
                let results = search_state.data.clone();
                let search_loading = search_state.loading;
                let error = search_state.error.clone();
                let root_entity = root_entity.clone();
                move |_, cx| {
                    let params = use_params(cx);
                    let keyword = params
                        .get("keywords")
                        .map(|value| value.as_ref().to_string())
                        .unwrap_or_default();
                    Self::render_search_route(
                        &keyword,
                        search_loading,
                        error.as_deref(),
                        &results,
                        &root_entity,
                    )
                }
            }))
            .child(Route::new().path("daily/songs").element({
                let daily_tracks = daily_tracks_state.data.clone();
                let daily_loading = daily_tracks_state.loading;
                let daily_error = daily_tracks_state.error.clone();
                let root_entity = root_entity.clone();
                move |_, _| {
                    let rows = daily_tracks
                        .iter()
                        .cloned()
                        .map(|track| {
                            let play_track = search::SearchSong {
                                id: track.id,
                                name: track.name.clone(),
                                artists: track.artists.clone(),
                            };
                            let queue_track = play_track.clone();
                            let root_for_play = root_entity.clone();
                            let root_for_queue = root_entity.clone();
                            playlist::track_row(
                                playlist::PlaylistTrackRow {
                                    id: track.id,
                                    name: track.name,
                                    artists: track.artists,
                                    cover_url: track.cover_url,
                                },
                                move |cx| {
                                    root_for_play.update(cx, |this, _| {
                                        this.queue_kernel_command(AppCommand::EnqueueSongAndPlay(
                                            SongInput {
                                                id: play_track.id,
                                                name: play_track.name.clone(),
                                                artists: play_track.artists.clone(),
                                            },
                                        ))
                                    });
                                },
                                move |cx| {
                                    root_for_queue.update(cx, |this, _| {
                                        this.queue_kernel_command(AppCommand::EnqueueSongOnly(
                                            SongInput {
                                                id: queue_track.id,
                                                name: queue_track.name.clone(),
                                                artists: queue_track.artists.clone(),
                                            },
                                        ))
                                    });
                                },
                            )
                        })
                        .collect::<Vec<_>>();

                    let replace_button = if daily_tracks.is_empty() {
                        None
                    } else {
                        let track_id = daily_tracks.first().map(|track| track.id);
                        let root_for_replace = root_entity.clone();
                        Some(
                            button::primary_pill("替换队列并播放")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    root_for_replace.update(cx, |this, _| {
                                        this.queue_kernel_command(
                                            AppCommand::ReplaceQueueFromDailyTracks(track_id),
                                        )
                                    });
                                })
                                .into_any_element(),
                        )
                    };

                    daily_tracks::render(
                        daily_loading,
                        daily_error.as_deref(),
                        rows,
                        replace_button,
                    )
                }
            }))
            .child(Route::new().path("next").element({
                let player_entity = player_entity.clone();
                let root_entity = root_entity.clone();
                let page_scroll_handle = page_scroll_handle.clone();
                move |_, cx| {
                    let player_snapshot = player_entity.read(cx).clone();
                    let current_track = player_snapshot.current_item().cloned();
                    let current_index = player_snapshot.current_index.unwrap_or(0);
                    let upcoming = player_snapshot
                        .queue
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index > current_index)
                        .map(|(_, item)| item.clone())
                        .collect::<Vec<_>>();

                    let queue_list = if upcoming.is_empty() {
                        None
                    } else {
                        let upcoming = Arc::new(upcoming);
                        let heights = Arc::new(vec![px(88.); upcoming.len()]);
                        let root_for_list = root_entity.clone();
                        let list = virtual_list::v_virtual_list(
                            ("next-queue", upcoming.len()),
                            heights,
                            move |visible_range, _, _| {
                                visible_range
                                    .map(|index| {
                                        let item = upcoming[index].clone();
                                        let play_root_entity = root_for_list.clone();
                                        let remove_root_entity = root_for_list.clone();
                                        let item_id = item.id;
                                        nekowg::div().pb(px(8.)).child(next::queue_row(
                                            item,
                                            move |cx| {
                                                play_root_entity.update(cx, |this, _| {
                                                    this.queue_kernel_command(
                                                        AppCommand::PlayQueueItem(item_id),
                                                    )
                                                });
                                            },
                                            move |cx| {
                                                remove_root_entity.update(cx, |this, _| {
                                                    this.queue_kernel_command(
                                                        AppCommand::RemoveQueueItem(item_id),
                                                    )
                                                });
                                            },
                                        ))
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .with_external_viewport_scroll(&page_scroll_handle)
                        .with_sizing_behavior(ListSizingBehavior::Infer)
                        .with_overscan(2)
                        .w_full();
                        Some(list.into_any_element())
                    };

                    let clear_button = if player_snapshot.queue.is_empty() {
                        None
                    } else {
                        let clear_root_entity = root_entity.clone();
                        Some(
                            button::pill_base("清空队列")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    clear_root_entity.update(cx, |this, _| {
                                        this.queue_kernel_command(AppCommand::ClearQueue)
                                    });
                                })
                                .into_any_element(),
                        )
                    };

                    next::render(current_track, clear_button, queue_list)
                }
            }))
            .child(Route::new().path("playlist/{id}").element({
                let root_entity = root_entity.clone();
                let playlist_pages = playlist_state_state.data.clone();
                let playlist_loading = playlist_state_state.loading;
                let playlist_error = playlist_state_state.error.clone();
                let page_scroll_handle = page_scroll_handle.clone();
                move |_, cx| {
                    let params = use_params(cx);
                    let playlist_id = params
                        .get("id")
                        .map(|value| value.as_ref().to_string())
                        .unwrap_or_else(|| "0".to_string());
                    let playlist_id_num = playlist_id.parse::<i64>().ok().unwrap_or_default();
                    let playlist_page = playlist_pages.get(&playlist_id_num).cloned();
                    let playlist_rows = playlist_page.as_ref().and_then(|page| {
                        if page.tracks.is_empty() {
                            return None;
                        }
                        let tracks = Arc::new(page.tracks.clone());
                        let heights = Arc::new(vec![px(84.); tracks.len()]);
                        let root_for_list = root_entity.clone();
                        let list = virtual_list::v_virtual_list(
                            ("playlist-tracks", page.id.unsigned_abs() as usize),
                            heights,
                            move |visible_range, _, _| {
                                visible_range
                                    .map(|index| {
                                        let track = tracks[index].clone();
                                        let play_track = search::SearchSong {
                                            id: track.id,
                                            name: track.name.clone(),
                                            artists: track.artists.clone(),
                                        };
                                        let queue_track = play_track.clone();
                                        let root_for_play = root_for_list.clone();
                                        let root_for_queue = root_for_list.clone();
                                        nekowg::div().pb(px(8.)).child(playlist::track_row(
                                            track,
                                            move |cx| {
                                                root_for_play.update(cx, |this, _| {
                                                    this.queue_kernel_command(
                                                        AppCommand::EnqueueSongAndPlay(SongInput {
                                                            id: play_track.id,
                                                            name: play_track.name.clone(),
                                                            artists: play_track.artists.clone(),
                                                        }),
                                                    )
                                                });
                                            },
                                            move |cx| {
                                                root_for_queue.update(cx, |this, _| {
                                                    this.queue_kernel_command(
                                                        AppCommand::EnqueueSongOnly(SongInput {
                                                            id: queue_track.id,
                                                            name: queue_track.name.clone(),
                                                            artists: queue_track.artists.clone(),
                                                        }),
                                                    )
                                                });
                                            },
                                        ))
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .with_external_viewport_scroll(&page_scroll_handle)
                        .with_sizing_behavior(ListSizingBehavior::Infer)
                        .with_overscan(2)
                        .w_full();
                        Some(list.into_any_element())
                    });
                    let replace_queue_button = playlist_page.as_ref().and_then(|page| {
                        if page.tracks.is_empty() {
                            return None;
                        }
                        let root_for_replace = root_entity.clone();
                        Some(
                            button::primary_pill("替换队列并播放")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    root_for_replace.update(cx, |this, _| {
                                        this.queue_kernel_command(
                                            AppCommand::ReplaceQueueFromPlaylist(playlist_id_num),
                                        )
                                    });
                                })
                                .into_any_element(),
                        )
                    });

                    playlist::render(
                        &playlist_id,
                        playlist_loading,
                        playlist_error.as_deref(),
                        playlist_page.as_ref(),
                        playlist_rows,
                        replace_queue_button,
                    )
                }
            }))
            .child(Route::new().path("settings").element({
                let root_entity = root_entity.clone();
                let close_behavior_label = close_behavior_label.clone();
                move |_, _| {
                    settings::render(
                        settings::SettingsViewModel {
                            close_behavior_label: close_behavior_label.clone().into(),
                        },
                        {
                            let root_entity = root_entity.clone();
                            move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::SetCloseBehavior(
                                        CloseBehavior::HideToTray,
                                    ));
                                });
                            }
                        },
                        {
                            let root_entity = root_entity.clone();
                            move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::SetCloseBehavior(
                                        CloseBehavior::Ask,
                                    ));
                                });
                            }
                        },
                        {
                            let root_entity = root_entity.clone();
                            move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::SetCloseBehavior(
                                        CloseBehavior::Exit,
                                    ));
                                });
                            }
                        },
                    )
                }
            }))
            .child(Route::new().path("login").element({
                let root_entity = root_entity.clone();
                let model = login_model.clone();
                move |_, _| {
                    login::render(
                        model.clone(),
                        {
                            let root_entity = root_entity.clone();
                            move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::GenerateLoginQr);
                                });
                            }
                        },
                        {
                            let root_entity = root_entity.clone();
                            move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::StopLoginQrPolling);
                                });
                            }
                        },
                        {
                            let root_entity = root_entity.clone();
                            move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::EnsureGuestSession);
                                });
                            }
                        },
                        {
                            let root_entity = root_entity.clone();
                            move |cx| {
                                root_entity.update(cx, |this, _| {
                                    this.queue_kernel_command(AppCommand::RefreshLoginToken);
                                });
                            }
                        },
                    )
                }
            }))
            .render(cx)
    }
}






