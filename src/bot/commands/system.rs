use anyhow::Context;
use matrix_sdk::{
    room::Joined,
    ruma::{events::room::message::RoomMessageEventContent, UserId},
};

use crate::db::queries;

use super::ErrList;

pub async fn exec(room: &Joined, user: &UserId) -> anyhow::Result<ErrList> {
    let members = queries::list_identities(user.as_str())
        .await
        .context("Error getting members from user")?;
    let mut msg = "## System Memebers\n\n".to_owned();
    for member in members {
        let info = queries::get_identity(user.as_str(), &member)
            .await
            .context(format!("Error getting info for member {member}"))?;
        msg += &format!(
            "- {} (`{}`)",
            info.display_name.as_ref().unwrap_or_else(|| &info.name),
            info.activators.join(","),
        );
    }
    room.send(RoomMessageEventContent::notice_markdown(msg), None)
        .await
        .context("Error sending reply")?;
    Ok(vec![])
}
