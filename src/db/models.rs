#[derive(sqlx::FromRow)]
pub struct Identity {
    pub mxid: String,
    pub name: String,
    pub display_name: String,
    pub avatar: String,
    #[sqlx(default)]
    pub activators: Vec<String>,
}
