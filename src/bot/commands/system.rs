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
    if members.is_empty() {
        room.send(
            RoomMessageEventContent::text_markdown(
                "You're account has no system members yet. Create some using `!member new [nam]`",
            ),
            None,
        )
        .await
        .context("Error sending reply")?;
        return Ok(vec![]);
    }
    let mut msg = "## System Memebers\n\n".to_owned();
    for member in members {
        let info = queries::get_identity(user.as_str(), &member)
            .await
            .context(format!("Error getting info for member {member}"))?;
        msg += &format!("- {}", info.display_name.as_ref().unwrap_or(&info.name));
        if !info.activators.is_empty() {
            msg += &format!(" (`{}`)", info.activators.join(","));
        }
        msg += "\n";
    }
    room.send(RoomMessageEventContent::text_markdown(msg), None)
        .await
        .context("Error sending reply")?;
    Ok(vec![])
}
