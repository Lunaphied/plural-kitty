use anyhow::Context;
use matrix_sdk::{
    room::Joined,
    ruma::{events::room::message::RoomMessageEventContent, UserId},
};

use crate::db::queries;

use super::ErrList;

pub async fn exec(room: &Joined, user: &UserId) -> anyhow::Result<ErrList> {
    let members = queries::list_members(user.as_str())
        .await
        .context("Error getting members from user")?;
    if members.is_empty() {
        room.send(
            RoomMessageEventContent::text_markdown(
                "This account has no system members yet. Create some using `!member new [name]`",
            ),
            None,
        )
        .await
        .context("Error sending reply")?;
        return Ok(vec![]);
    }
    let mut msg = "#### System Members\n\n".to_owned();
    for member in members {
        let info = queries::get_member(user.as_str(), &member)
            .await
            .context(format!("Error getting info for member {member}"))?;
        msg += &format!("- {}", info.display_name.as_ref().unwrap_or(&info.name));
        if !info.activators.is_empty() {
            msg += &format!(" (`{}`)", info.activators.join(","));
        }
        msg += "\n";
    }
    let current_fronter = queries::get_current_fronter(user.as_str())
        .await
        .with_context(|| format!("Error getting current fronter for {user}"))?;
    if let Some(current_fronter) = current_fronter {
        msg += &format!("\n**Current fronter: {}**", current_fronter.name);
    } else {
        msg += "\n**No current fronter**";
    }
    room.send(RoomMessageEventContent::text_markdown(msg), None)
        .await
        .context("Error sending reply")?;
    Ok(vec![])
}
