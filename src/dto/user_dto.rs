use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub username: String,
    pub normalized_username: String,
    pub registration_date: String,
    pub is_verified: bool,
    pub quota_bytes: i32,
    pub is_locked: bool,
    pub used_bytes: i32,
    pub profile: Option<Profile>,
    pub patreon_data: Option<PatreonData>,
}

impl User {
    pub fn is_patron(&self) -> bool {
        self.patreon_data.as_ref().map_or(false, |p| p.is_patreon_supporter)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub icon_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatreonData {
    pub is_patreon_supporter: bool,
    pub last_patreon_pledge_cents: i32,
    pub last_total_cents: i32,
    pub last_total_units: i32,
    pub minimum_total_units: i32,
    pub external_cents: i32,
    pub last_external_cents: i32,
    pub has_supported: bool,
    pub last_is_anorak: bool,
    pub priority_issue: i32,
    pub last_plus_activation_time: String,
    pub last_activation_time: String,
    pub last_plus_pledge_amount: i32,
    pub last_paid_pledge_amount: i32,
    pub account_name: String,
    pub current_account_type: i32,
    pub current_account_cents: i32,
    pub pledged_account_type: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AbridgedUser {
    pub registration_date: String,
    pub is_patron: bool,
}

impl From<User> for AbridgedUser {
    fn from(user: User) -> Self {
        AbridgedUser {
            is_patron: user.is_patron(),
            registration_date: user.registration_date,
        }
    }
}