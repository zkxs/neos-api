use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStatus {
    pub online_status: String,
    #[serde(with = "crate::dto::custom_serializer::iso_8601")]
    pub last_status_change: DateTime<Utc>,
    pub current_session_access_level: i32,
    pub current_session_hidden: bool,
    pub current_hosting: bool,
    pub compatibility_hash: String,
    pub neos_version: String,
    pub output_device: String,
    pub is_mobile: bool,
}
