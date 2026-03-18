use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::api::common::models::{AlbumDto, PlaylistDto, TrackDto};
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Song,
    Album,
    Artist,
    Playlist,
}

impl SearchType {
    pub const fn code(self) -> u32 {
        match self {
            Self::Song => 1,
            Self::Album => 10,
            Self::Artist => 100,
            Self::Playlist => 1000,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SearchArtistDto {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
    #[serde(default, rename = "img1v1Url")]
    pub img1v1_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SearchResult {
    pub songs: Vec<TrackDto>,
    pub artists: Vec<SearchArtistDto>,
    pub albums: Vec<AlbumDto>,
    pub playlists: Vec<PlaylistDto>,
    pub has_more: bool,
    pub song_count: u64,
    pub artist_count: u64,
    pub album_count: u64,
    pub playlist_count: u64,
}

impl<'de> Deserialize<'de> for SearchResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Self {
            songs: parse_vec(&value, "songs").map_err(D::Error::custom)?,
            artists: parse_vec(&value, "artists").map_err(D::Error::custom)?,
            albums: parse_vec(&value, "albums").map_err(D::Error::custom)?,
            playlists: parse_vec(&value, "playlists").map_err(D::Error::custom)?,
            has_more: value
                .get("hasMore")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            song_count: parse_count(&value, "songCount"),
            artist_count: parse_count(&value, "artistCount"),
            album_count: parse_count(&value, "albumCount"),
            playlist_count: parse_count(&value, "playlistCount"),
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SearchResponse {
    pub code: i64,
    #[serde(default)]
    pub result: SearchResult,
}

pub struct SearchRequest {
    pub keywords: String,
    pub search_type: SearchType,
    pub offset: u32,
    pub limit: u32,
}

impl SearchRequest {
    pub fn new(keywords: impl Into<String>, search_type: SearchType) -> Self {
        Self {
            keywords: keywords.into(),
            search_type,
            offset: 0,
            limit: 30,
        }
    }
}

impl ApiRequest for SearchRequest {
    type Response = SearchResponse;

    fn endpoint(&self) -> &'static str {
        "/api/search/get"
    }

    fn payload(&self) -> Value {
        json!({
            "s": self.keywords,
            "type": self.search_type.code(),
            "offset": self.offset,
            "limit": self.limit,
        })
    }
}

fn parse_vec<T>(value: &Value, key: &str) -> Result<Vec<T>, serde_json::Error>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(
        value
            .get(key)
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    )
}

fn parse_count(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|count| u64::try_from(count).ok()))
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{SearchRequest, SearchResponse, SearchType};
    use crate::api::request::ApiRequest;

    #[test]
    fn search_payload_contains_requested_type() {
        let req = SearchRequest::new("hello", SearchType::Album);
        let payload = req.payload();
        assert_eq!(req.endpoint(), "/api/search/get");
        assert_eq!(payload["s"].as_str(), Some("hello"));
        assert_eq!(payload["type"].as_i64(), Some(10));
    }

    #[test]
    fn search_result_allows_duplicate_playlist_creator_keys() {
        let raw = r#"{
            "code": 200,
            "result": {
                "playlists": [
                    {
                        "id": 1,
                        "name": "Test",
                        "creator": {"nickname": "A"},
                        "creator": {"nickname": "B"}
                    }
                ]
            }
        }"#;

        let parsed: SearchResponse =
            serde_json::from_str(raw).expect("search response should deserialize");

        assert_eq!(parsed.result.playlists.len(), 1);
        assert_eq!(
            parsed.result.playlists[0]
                .creator
                .as_ref()
                .and_then(|creator| creator.name.as_deref()),
            Some("B")
        );
    }
}
