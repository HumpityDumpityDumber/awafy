use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub id: String,
    pub name: String,
    pub stat: Stat,
    pub image: Image,
    pub demographics: Demographics,
    pub attribute: Attribute,
    pub created_at: i64,
    pub cached_at: i64,
    pub uploaded_at: i64,
    pub playlist_updated_at: i64,
    pub updated_at: i64,
    pub room_first_started_at: i64,
    pub is_official: bool,
    pub is_essential_liver: bool,
    pub is_picked_out_liver: bool,
    pub is_contractual: bool,
    pub is_deleted: bool,
    pub auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    pub favorited: i64,
    pub playlist: PlaylistStats,
    pub favorite: FavoriteStats,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistStats {
    pub total: i64,
    pub published: i64,
    pub played: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FavoriteStats {
    pub users: i64,
    pub playlists: i64,
    pub tracks: i64,
    pub artists: i64,
    pub albums: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub dominant_color: String,
    pub vibrant_color: String,
    pub color: ColorData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorData {
    pub dominant_color: String,
    pub vibrant: VibrantColors,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VibrantColors {
    pub light_muted: String,
    pub dark_muted: String,
    pub vibrant: String,
    pub light_vibrant: String,
    pub dark_vibrant: String,
    pub muted: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Demographics {
    pub gender: VisibilityValue<i64>,
    pub birth_date: BirthDate,
    pub prefecture_jp: VisibilityValue<i64>,
    pub country: VisibilityValue<i64>,
    pub geo: Geo,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityValue<T> {
    pub is_visible: bool,
    pub value: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BirthDate {
    pub is_visible: bool,
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Geo {
    pub is_visible: bool,
    pub latitude: String,
    pub longitude: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribute {
    pub id: String,
    pub genre: String,
    pub mood: String,
    pub content_type: String,
    pub audience_type: String,
    pub frequent_artist_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthData {
    pub awa: AwaId,
    pub refresh: RefreshToken,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaId {
    pub id: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshToken {
    pub refresh_token: String,
    pub access_token: String,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Code {
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Credentials {
    pub access_token: String,
    pub token_expiry: i64,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub device_id: String,
}
