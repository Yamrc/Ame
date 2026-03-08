use gpui::{AnyElement, Entity, ListSizingBehavior, MouseButton, ScrollHandle, prelude::*, px};
use gpui_router::{Route, Routes, use_params};
use std::{collections::HashMap, sync::Arc};

use crate::component::{button, virtual_list};
use crate::entity::app::CloseBehavior;
use crate::entity::player::PlayerEntity;
use crate::kernel::{AppCommand, SongInput};
use crate::view::{discover, home, library, login, next, playlist, search, settings};

use super::RootView;

pub(super) struct RoutesModel {
    pub home_playlists: Vec<library::LibraryPlaylistCard>,
    pub is_user_logged_in: bool,
    pub home_loading: bool,
    pub home_error: Option<String>,
    pub discover_playlists: Vec<library::LibraryPlaylistCard>,
    pub discover_loading: bool,
    pub discover_error: Option<String>,
    pub search_results: Vec<search::SearchSong>,
    pub search_loading: bool,
    pub search_error: Option<String>,
    pub library_playlists: Vec<library::LibraryPlaylistCard>,
    pub library_loading: bool,
    pub library_error: Option<String>,
    pub playlist_pages: HashMap<i64, playlist::PlaylistPage>,
    pub page_scroll_handle: ScrollHandle,
    pub playlist_loading: bool,
    pub playlist_error: Option<String>,
    pub auth_account_summary: Option<String>,
    pub login_model: login::LoginViewModel,
    pub close_behavior_label: String,
}

impl RootView {
    fn build_home_featured_rows(
        playlists: &[library::LibraryPlaylistCard],
        root_entity: &Entity<RootView>,
        is_user_logged_in: bool,
    ) -> Vec<AnyElement> {
        let mut rows = Vec::with_capacity(2);

        let daily_root = root_entity.clone();
        let daily_playlist_id = playlists.first().map(|item| item.id);
        rows.push(home::featured_card(
            home::HomePlaylistCard {
                id: daily_playlist_id.unwrap_or_default(),
                kind: home::HomeFeaturedKind::Daily,
                name: "每日推荐歌单".to_string(),
                subtitle: "根据你的口味更新".to_string(),
                cover_url: playlists.first().and_then(|item| item.cover_url.clone()),
            },
            move |cx| {
                daily_root.update(cx, |this, _| {
                    if is_user_logged_in {
                        if let Some(playlist_id) = daily_playlist_id {
                            this.queue_kernel_command(AppCommand::OpenLibraryPlaylist(playlist_id));
                        } else {
                            this.queue_kernel_command(AppCommand::Navigate("/explore".to_string()));
                        }
                    } else {
                        this.queue_kernel_command(AppCommand::Navigate("/login".to_string()));
                    }
                });
            },
        ));

        let fm_root = root_entity.clone();
        rows.push(home::featured_card(
            home::HomePlaylistCard {
                id: 0,
                kind: home::HomeFeaturedKind::Fm,
                name: "私人 FM".to_string(),
                subtitle: "连续播放你可能喜欢的音乐".to_string(),
                cover_url: playlists.get(1).and_then(|item| item.cover_url.clone()),
            },
            move |cx| {
                fm_root.update(cx, |this, _| {
                    let target = if is_user_logged_in {
                        "/library"
                    } else {
                        "/login"
                    };
                    this.queue_kernel_command(AppCommand::Navigate(target.to_string()));
                });
            },
        ));

        rows
    }

    fn build_home_playlist_rows(
        playlists: &[library::LibraryPlaylistCard],
        root_entity: &Entity<RootView>,
    ) -> Vec<AnyElement> {
        playlists
            .iter()
            .take(15)
            .cloned()
            .map(|item| {
                let playlist_id = item.id;
                let root_entity = root_entity.clone();
                home::playlist_card(
                    home::HomePlaylistCard {
                        id: item.id,
                        kind: home::HomeFeaturedKind::Playlist,
                        name: item.name,
                        subtitle: format!("{} 首 · by {}", item.track_count, item.creator_name),
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
        playlists: &[library::LibraryPlaylistCard],
        root_entity: &Entity<RootView>,
    ) -> Vec<AnyElement> {
        playlists
            .iter()
            .cloned()
            .map(|item| {
                let playlist_id = item.id;
                let root_entity = root_entity.clone();
                library::playlist_row(item, move |cx| {
                    root_entity.update(cx, |this, _| {
                        this.queue_kernel_command(AppCommand::OpenLibraryPlaylist(playlist_id))
                    });
                })
            })
            .collect()
    }

    fn build_discover_rows(
        playlists: &[library::LibraryPlaylistCard],
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
        root_entity: Entity<RootView>,
        player_entity: Entity<PlayerEntity>,
        model: RoutesModel,
    ) -> AnyElement {
        let RoutesModel {
            home_playlists,
            is_user_logged_in,
            home_loading,
            home_error,
            discover_playlists,
            discover_loading,
            discover_error,
            search_results,
            search_loading,
            search_error,
            library_playlists,
            library_loading,
            library_error,
            playlist_pages,
            page_scroll_handle,
            playlist_loading,
            playlist_error,
            auth_account_summary,
            login_model,
            close_behavior_label,
        } = model;

        Routes::new()
            .basename("/")
            .child(Route::new().index().element({
                let root_entity = root_entity.clone();
                let home_playlists = home_playlists.clone();
                let home_error = home_error.clone();
                move |_, _| {
                    home::render(
                        home_loading,
                        home_error.as_deref(),
                        Self::build_home_featured_rows(
                            &home_playlists,
                            &root_entity,
                            is_user_logged_in,
                        ),
                        Self::build_home_playlist_rows(&home_playlists, &root_entity),
                    )
                }
            }))
            .child(Route::new().path("explore").element({
                let root_entity = root_entity.clone();
                let discover_playlists = discover_playlists.clone();
                let discover_error = discover_error.clone();
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
                let library_playlists = library_playlists.clone();
                let library_error = library_error.clone();
                let auth_account_summary = auth_account_summary.clone();
                move |_, _| {
                    let rows = Self::build_library_rows(&library_playlists, &root_entity);
                    let title = auth_account_summary
                        .as_deref()
                        .filter(|summary| !summary.trim().is_empty())
                        .map(|summary| format!("{summary} 的音乐库"))
                        .unwrap_or_else(|| "我的音乐库".to_string());
                    library::render(&title, library_loading, library_error.as_deref(), rows)
                }
            }))
            .child(Route::new().path("search").element({
                let results = search_results.clone();
                let error = search_error.clone();
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
                let results = search_results.clone();
                let error = search_error.clone();
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
            .child(Route::new().path("next").element({
                let player_entity = player_entity.clone();
                let root_entity = root_entity.clone();
                move |_, cx| {
                    let player_snapshot = player_entity.read(cx).clone();
                    let current_track = player_snapshot.current_item().cloned();
                    let current_index = player_snapshot.current_index.unwrap_or(0);

                    let queue_rows = player_snapshot
                        .queue
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index > current_index)
                        .map(|(_, item)| item.clone())
                        .map(|item| {
                            let play_root_entity = root_entity.clone();
                            let remove_root_entity = root_entity.clone();
                            let item_id = item.id;

                            next::queue_row(
                                item,
                                move |cx| {
                                    play_root_entity.update(cx, |this, _| {
                                        this.queue_kernel_command(AppCommand::PlayQueueItem(
                                            item_id,
                                        ))
                                    });
                                },
                                move |cx| {
                                    remove_root_entity.update(cx, |this, _| {
                                        this.queue_kernel_command(AppCommand::RemoveQueueItem(
                                            item_id,
                                        ))
                                    });
                                },
                            )
                        })
                        .collect();

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

                    next::render(current_track, clear_button, queue_rows)
                }
            }))
            .child(Route::new().path("playlist/{id}").element({
                let root_entity = root_entity.clone();
                let playlist_pages = playlist_pages.clone();
                let playlist_error = playlist_error.clone();
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
                                        gpui::div().pb(px(8.)).child(playlist::track_row(
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

                    playlist::render(
                        &playlist_id,
                        playlist_loading,
                        playlist_error.as_deref(),
                        playlist_page.as_ref(),
                        playlist_rows,
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
            .into_any_element()
    }
}
