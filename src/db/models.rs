#[derive(sqlx::FromRow)]
pub struct Identity {
    pub mxid: String,
    pub name: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    #[sqlx(default)]
    pub activators: Vec<String>,
}

#[derive(sqlx::FromRow)]
pub struct ActivatorInfo {
    pub name: String,
    pub value: String,
}
