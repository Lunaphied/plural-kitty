#[derive(sqlx::FromRow)]
pub struct Identity {
    pub mxid: String,
    pub name: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub activators: Vec<String>,
    pub track_account: bool,
}

#[derive(sqlx::FromRow)]
pub struct ActivatorInfo {
    pub name: String,
    pub value: String,
}
