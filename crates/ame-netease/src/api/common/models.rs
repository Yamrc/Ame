use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ArtistDto {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CreatorDto {
    #[serde(default, rename = "nickname", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "userId")]
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PlaylistDto {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "trackCount", alias = "track_count")]
    pub track_count: Option<u64>,
    #[serde(default)]
    pub creator: Option<CreatorDto>,
    #[serde(default, rename = "creatorName")]
    pub creator_name: Option<String>,
    #[serde(default, rename = "creatorId")]
    pub creator_id: Option<i64>,
    #[serde(default)]
    pub subscribed: Option<bool>,
    #[serde(default, rename = "specialType")]
    pub special_type: Option<i64>,
    #[serde(default, rename = "coverImgUrl", alias = "picUrl")]
    pub cover_img_url: Option<String>,
    #[serde(default, rename = "updateFrequency")]
    pub update_frequency: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AlbumDto {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "picUrl", alias = "coverImgUrl")]
    pub pic_url: Option<String>,
    #[serde(default)]
    pub artist: Option<ArtistDto>,
    #[serde(default)]
    pub artists: Vec<ArtistDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackAlbumDto {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackDto {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub alia: Vec<String>,
    #[serde(default, rename = "tns")]
    pub tns: Vec<String>,
    #[serde(default, rename = "transNames")]
    pub trans_names: Vec<String>,
    #[serde(default, alias = "ar")]
    pub artists: Vec<ArtistDto>,
    #[serde(default, alias = "al")]
    pub album: TrackAlbumDto,
    #[serde(default, rename = "duration", alias = "dt")]
    pub duration_ms: Option<u64>,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UserAccountDto {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default, rename = "userName")]
    pub user_name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UserProfileDto {
    #[serde(default, rename = "userId")]
    pub user_id: Option<i64>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default, rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}
