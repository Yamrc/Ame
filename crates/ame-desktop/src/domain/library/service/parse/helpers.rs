use ame_netease::api::common::models::{ArtistDto, TrackDto};
use tracing::warn;

pub(in crate::domain::library::service::parse) fn compact_cover_url(
    raw: Option<&str>,
    size: u32,
) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.contains("param=") {
        return Some(raw.to_string());
    }
    let separator = if raw.contains('?') { '&' } else { '?' };
    Some(format!("{raw}{separator}param={size}y{size}"))
}

pub(in crate::domain::library::service::parse) fn parse_artist_names(
    artists: &[ArtistDto],
) -> String {
    let artists = artists
        .iter()
        .filter_map(|artist| artist.name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    if artists.is_empty() {
        "未知艺人".to_string()
    } else {
        artists
    }
}

pub(in crate::domain::library::service::parse) fn parse_track_alias(
    track: &TrackDto,
) -> Option<String> {
    let alias = track
        .tns
        .iter()
        .chain(track.trans_names.iter())
        .chain(track.alia.iter())
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    (!alias.is_empty()).then_some(alias)
}

pub(in crate::domain::library::service::parse) fn display_name(
    raw: Option<&str>,
    fallback: &str,
) -> String {
    raw.filter(|name| !name.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| fallback.to_string())
}

pub(in crate::domain::library::service::parse) fn sanitize_name(
    raw: Option<&str>,
) -> Option<String> {
    raw.map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .and_then(|name| {
            name.split_once('\0')
                .map(|(name, _)| name.to_string())
                .or(Some(name))
        })
}

pub(in crate::domain::library::service) fn parse_track_count_or_zero(
    raw: u64,
    source: &'static str,
    entity_id: i64,
) -> u32 {
    match u32::try_from(raw) {
        Ok(value) => value,
        Err(err) => {
            warn!("invalid track_count from {source}, id={entity_id}, value={raw}: {err}");
            0
        }
    }
}
