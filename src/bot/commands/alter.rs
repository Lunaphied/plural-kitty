use anyhow::anyhow;
use anyhow::bail;
use matrix_sdk::room::Joined;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::UserId;

use crate::bot::parser::Cmd;
use crate::db::queries;

use super::ErrList;

pub async fn exec(mut cmd: Cmd, room: &Joined, user: &UserId) -> anyhow::Result<ErrList> {
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
            "show" => show_identity(room, user, &name).await?,
            s => bail!("Unkown command {s}"),
        }
    }
    Ok(vec![])
}

async fn new_ident(mut cmd: Cmd, room: &Joined, user: &UserId) -> anyhow::Result<()> {
    let name = cmd.pop_word().ok_or_else(|| anyhow!("Give name plz"))?;
    queries::create_identity(user.as_str(), &name).await?;
    room.send(
        RoomMessageEventContent::text_markdown(format!("Created idenity `{name}`")),
        None,
    )
    .await?;
    Ok(())
}

async fn add_display_name(
    mut cmd: Cmd,
    room: &Joined,
    user: &UserId,
    name: &str,
) -> anyhow::Result<()> {
    let display_name = cmd.pop_word().ok_or_else(|| anyhow!("Give name plz"))?;
    queries::add_display_name(user.as_str(), name, &display_name).await?;
    room.send(
        RoomMessageEventContent::text_markdown(format!("Set display name to \"{display_name}\"")),
        None,
    )
    .await?;
    Ok(())
}

async fn show_identity(room: &Joined, user: &UserId, name: &str) -> anyhow::Result<()> {
    let idenity = queries::get_idenity(user.as_str(), name).await?;
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
