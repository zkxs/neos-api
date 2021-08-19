use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::dto::cache_user_dto::AbridgedUser;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub name: Option<String>,
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
    pub session_users: Vec<SessionUser>,
    pub thumbnail: Option<String>,
    pub joined_users: i32,
    pub active_users: i32,
    pub max_users: i32,
    pub mobile_friendly: bool,
    #[serde(with = "crate::dto::custom_serializer::iso_8601")]
    pub session_begin_time: DateTime<Utc>,
    #[serde(with = "crate::dto::custom_serializer::iso_8601")]
    pub last_update: DateTime<Utc>,
    pub away_since: Option<String>, // actually an Option<DateTime<Utc>>. but a bitch to deserialize so fuck it
    pub access_level: String,
    pub has_ended: bool,
    pub is_valid: bool,
}

impl Session {
    pub fn headed_users(self) -> Box<dyn Iterator<Item=SessionUser>> {
        if self.headless_host {
            let host_user_id = self.host_user_id;
            let host_username = self.host_username;
            let iter = self.session_users.into_iter()
                .filter(move |u| !(host_user_id == u.user_id && host_username == u.username));
            Box::new(iter)
        } else {
            let iter = self.session_users.into_iter();
            Box::new(iter)
        }
    }
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
pub struct SessionUser {
    pub username: String,
    #[serde(rename = "userID")]
    pub user_id: Option<String>,
    pub is_present: bool,
}

pub struct SessionWithHostInfo {
    pub session: Session,
    pub host_info: Option<AbridgedUser>,
}
