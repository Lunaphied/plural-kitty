pub struct Identity {
    pub mxid: String,
    pub name: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub activators: Vec<String>,
    pub track_account: bool,
}

