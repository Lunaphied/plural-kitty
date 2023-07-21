use matrix_sdk::room::Joined;
use matrix_sdk::ruma::UserId;

use crate::bot::parser::Cmd;

use super::ErrList;

pub async fn exec(mut cmd: Cmd, room: &Joined) -> anyhow::Result<ErrList> {
    Ok(vec![])
}

async fn create_ident(user: &UserId, name: &str) -> anyhow::Result<()> {
    Ok(())
}
