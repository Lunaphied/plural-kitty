pub struct Member {
    pub mxid: String,
    pub name: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub activators: Vec<String>,
    pub track_account: bool,
}

#[derive(sqlx::FromRow)]
pub struct ProfileInfo {
    #[sqlx(rename = "displayname")]
    pub display_name: String,
    #[sqlx(rename = "avatar_url")]
    pub avatar: String,
}
