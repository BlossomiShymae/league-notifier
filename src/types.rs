use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendResource {
    pub availability: String,
    pub game_name: String,
    pub game_tag: String,
    pub icon: i32,
    pub puuid: String,
    pub product: String,
}
