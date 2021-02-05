use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub name: String,
    pub description: Option<String>,
    pub corresponding_world_id: Option<World>,
    pub tags: Vec<String>,
    pub session_id: String,
    pub normalized_session_id: String,
    pub host_user_id: Option<String>,
    pub host_machine_id: String,
    pub host_username: String,
    pub compatibility_hash: String,
    pub universe_id: Option<String>,
    pub neos_version: String,
    pub headless_host: bool,
    #[serde(rename = "sessionURLs")]
    pub session_urls: Vec<String>,
    pub session_users: Vec<User>,
    pub thumbnail: Option<String>,
    pub joined_users: i32,
    pub active_users: i32,
    pub max_users: i32,
    pub mobile_friendly: bool,
    pub session_begin_time: String,
    pub last_update: String,
    pub away_since: Option<String>,
    pub access_level: String,
    pub has_ended: bool,
    pub is_valid: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub record_id: String,
    pub owner_id: String,
    pub is_valid: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub username: String,
    #[serde(rename = "userID")]
    pub user_id: Option<String>,
    pub is_present: bool,
}
