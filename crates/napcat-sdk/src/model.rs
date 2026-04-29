use serde::Deserialize;

/// Response from get_friend_list.
#[derive(Debug, Clone, Deserialize)]
pub struct FriendInfo {
    pub user_id: i64,
    pub nickname: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub sex: String,
    #[serde(default)]
    pub age: i32,
}

/// Response from get_group_list.
#[derive(Debug, Clone, Deserialize)]
pub struct GroupInfo {
    pub group_id: i64,
    pub group_name: String,
    #[serde(default)]
    pub group_remark: String,
    #[serde(default)]
    pub member_count: i32,
    #[serde(default)]
    pub max_member_count: i32,
}
