use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use matrix_sdk::room::Joined;
use matrix_sdk::ruma::events::room::message::{
    OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};
use matrix_sdk::ruma::UserId;

use crate::bot::parser::Cmd;
use crate::db::queries;

use super::ErrList;

pub async fn exec(
    mut cmd: Cmd,
    room: &Joined,
    event: &OriginalSyncRoomMessageEvent,
) -> anyhow::Result<ErrList> {
    let user = &event.sender;
    let first_arg = cmd.pop_word().ok_or_else(|| anyhow!("More args plz"))?;
    if first_arg == "new" {
        new_ident(cmd, room, user).await?;
    } else {
        let name = first_arg;
        let sub_command = cmd
            .pop_word()
            .ok_or_else(|| anyhow!("Please specify a subcommand"))?;
        match sub_command.as_str() {
            "displayname" | "dn" => add_display_name(cmd, room, user, &name).await?,
            "activator" | "act" => activator_cmd(cmd, room, user, &name).await?,
            "show" => show_identity(room, user, &name).await?,
            s => bail!("Unkown command {s}"),
        }
    }
    Ok(vec![])
}

async fn new_ident(mut cmd: Cmd, room: &Joined, user: &UserId) -> anyhow::Result<()> {
    let name = cmd.pop_word().ok_or_else(|| anyhow!("Give name plz"))?;
    queries::create_user(user.as_str()).await?;
    queries::create_identity(user.as_str(), &name).await?;
    room.send(
        RoomMessageEventContent::text_markdown(format!("Created idenity `{name}`")),
        None,
    )
    .await?;
    Ok(())
}

async fn add_display_name(
    cmd: Cmd,
    room: &Joined,
    user: &UserId,
    name: &str,
) -> anyhow::Result<()> {
    let display_name = cmd.into_string();
    if display_name.is_empty() {
        bail!("Giv name plz");
    }
    queries::add_display_name(user.as_str(), name, &display_name).await?;
    room.send(
        RoomMessageEventContent::text_markdown(format!("Set display name to `{display_name}`")),
        None,
    )
    .await?;
    Ok(())
}

async fn show_identity(room: &Joined, user: &UserId, name: &str) -> anyhow::Result<()> {
    let idenity = queries::get_identity(user.as_str(), name).await?;
    let message = format!(
        "## {}\n\n---\n\nDisplay name: {}\n\nAvatar: `{}`\n\nActivators: `{}`",
        idenity.name,
        idenity.display_name.as_deref().unwrap_or("`not set`"),
        idenity.avatar.as_deref().unwrap_or("not set"),
        idenity.activators.join(", "),
    );
    room.send(RoomMessageEventContent::text_markdown(message), None)
        .await?;
    Ok(())
}

async fn activator_cmd(
    mut cmd: Cmd,
    room: &Joined,
    user: &UserId,
    name: &str,
) -> anyhow::Result<()> {
    let sub_command = cmd
        .pop_word()
        .ok_or_else(|| anyhow!("Please specify a sub-command"))?;
    match sub_command.as_str() {
        "add" => {
            let activator = cmd.pop_word().ok_or_else(|| {
                anyhow!(
                    "!member [member] activator add needs the activation sequence as an arquement"
                )
            })?;
            if activator.starts_with('!') {
                bail!("activation sequence cannot start with `!`");
            }
            queries::add_activator(user.as_str(), name, &activator)
                .await
                .context("Error adding activator")?;
            let msg = format!("Added activator `{}` to {}", activator, name);
            room.send(RoomMessageEventContent::notice_markdown(msg), None)
                .await
                .context("Error sending reply")?;
        }
        unknown => bail!("Unkown sub-command `{unknown}`"),
    }
    Ok(())
}
