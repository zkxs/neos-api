use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// used for user cache
#[derive(Serialize, Deserialize, Clone)]
pub struct AbridgedUser {
    #[serde(with = "crate::dto::custom_serializer::iso_8601")]
    pub registration_date: DateTime<Utc>,
    pub is_patron: bool,
    pub is_mentor: bool,
    #[serde(with = "crate::dto::custom_serializer::iso_8601")]
    pub cache_time: DateTime<Utc>,
}
