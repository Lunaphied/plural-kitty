use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use matrix_sdk::room::Joined;
use matrix_sdk::ruma::api::client::media::create_content;
use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent, Relation, RoomMessageEventContent,
};
use matrix_sdk::ruma::events::room::MediaSource;
use matrix_sdk::ruma::events::{
    AnyMessageLikeEvent, AnyTimelineEvent, MessageLikeEvent, OriginalMessageLikeEvent,
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
        new_member(cmd, room, user).await?;
    } else {
        let name = first_arg;
        if !queries::member_exists(user.as_str(), &name).await? {
            bail!(
                "Memeber {name} does not exist.\n\nYou can create this member with `!m new {name}`"
            );
        }
        let sub_command = cmd
            .pop_word()
            .ok_or_else(|| anyhow!("Please specify a subcommand"))?;
        match sub_command.as_str() {
            "displayname" | "dn" => add_display_name(cmd, room, user, &name).await?,
            "activator" | "act" => activator_cmd(cmd, room, user, &name).await?,
            "avatar" | "av" => add_avatar(cmd, room, user, &name, event).await?,
            "trackaccount" | "ta" => toggle_track_acc(room, user, &name).await?,
            "show" => show_member(room, user, &name).await?,
            "remove" => remove_member(room, user, &name).await?,
            s => bail!("Unkown command {s}"),
        }
    }
    Ok(vec![])
}

async fn new_member(mut cmd: Cmd, room: &Joined, user: &UserId) -> anyhow::Result<()> {
    let name = cmd.pop_word().ok_or_else(|| anyhow!("Give name plz"))?;
    queries::create_user(user.as_str()).await?;
    queries::create_member(user.as_str(), &name).await?;
    room.send(
        RoomMessageEventContent::text_markdown(format!("Created member `{name}`")),
        None,
    )
    .await?;
    Ok(())
}

async fn remove_member(room: &Joined, user: &UserId, name: &str) -> anyhow::Result<()> {
    queries::remove_member(user.as_str(), name).await?;
    room.send(
        RoomMessageEventContent::text_markdown(format!("Removed member `{name}`")),
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
    let mut display_name = cmd.into_string();
    if display_name.is_empty() {
        bail!("Giv name plz");
    }
    if display_name.as_str() == "!clear" {
        queries::remove_display_name(user.as_str(), name).await?;
        room.send(
            RoomMessageEventContent::text_markdown("Cleared display name"),
            None,
        )
        .await?;
    } else {
        if display_name.as_str() == "!acc" {
            display_name = queries::get_synapse_profile(user.as_str())
                .await?
                .display_name;
        }
        queries::add_display_name(user.as_str(), name, &display_name).await?;
        room.send(
            RoomMessageEventContent::text_markdown(format!("Set display name to `{display_name}`")),
            None,
        )
        .await?;
    }
    Ok(())
}

async fn add_avatar(
    mut cmd: Cmd,
    room: &Joined,
    user: &UserId,
    name: &str,
    event: &OriginalSyncRoomMessageEvent,
) -> anyhow::Result<()> {
    if let Some(word) = cmd.pop_word() {
        if word.as_str() == "!clear" {
            queries::remove_avatar(user.as_str(), name).await?;
        } else if word.as_str() == "!acc" {
            let profile = queries::get_synapse_profile(user.as_str()).await?;
            queries::add_avatar(user.as_str(), name, &profile.avatar).await?;
        } else if word.starts_with("mxc://") {
            queries::add_avatar(user.as_str(), name, &word).await?;
        } else {
            bail!("Unkown argument `{word}`, must be `!clear`, `!acc`, or an mxc url");
        }
        room.send(RoomMessageEventContent::text_plain("Updated avatar"), None)
            .await?;
    } else if let Some(Relation::Reply { in_reply_to }) = &event.content.relates_to {
        let AnyTimelineEvent::MessageLike(AnyMessageLikeEvent::RoomMessage(
            MessageLikeEvent::Original(OriginalMessageLikeEvent {
                content: RoomMessageEventContent { msgtype: MessageType::Image(image_event), .. }, ..
            }),
        )) = room
            .event(&in_reply_to.event_id)
            .await
            .context("Error getting image event")?
            .event
            .deserialize()
            .context("Error deserializing image event")? else 
        {
            bail!("`!member [member] avatar must be sent in reply to an image");
        };
        let media_mxc = match image_event.source {
            MediaSource::Plain(mxc) => mxc,
            MediaSource::Encrypted(_) => {
                // Reupload encrypted images unencrypted
                let image = room
                    .client()
                    .media()
                    .get_file(image_event.clone(), false)
                    .await
                    .context("Error getting encrypted image")?
                    .ok_or_else(|| {
                        anyhow!("The message this command is replying to has not image file")
                    })?;
                let mut upload_req = create_content::v3::Request::new(image);
                if let Some(info) = image_event.info {
                    upload_req.content_type = info.mimetype;
                }
                room.client().send(upload_req, None).await?.content_uri
            }
        };
        queries::add_avatar(user.as_str(), name, media_mxc.as_str()).await?;
    } else {
        bail!("`!member [member] avatar must be sent in reply to an image");
    }
    Ok(())
}

async fn show_member(room: &Joined, user: &UserId, name: &str) -> anyhow::Result<()> {
    let member = queries::get_member(user.as_str(), name).await?;
    let message = format!(
        "## {}\n\n---\n\nDisplay name: {}\n\nAvatar: `{}`\n\nActivators: `{}`",
        member.name,
        member.display_name.as_deref().unwrap_or("`not set`"),
        member.avatar.as_deref().unwrap_or("not set"),
        member.activators.join(", "),
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
    if !queries::member_exists(user.as_str(), name).await? {
        bail!("Member {name} does not exist");
    }
    match sub_command.as_str() {
        "add" => {
            let activator = cmd
                .pop_word()
                .ok_or_else(|| {
                    anyhow!(
                    "!member [member] activator add needs the activation sequence as an arquement"
                )
                })?
                .to_lowercase();
            if activator.starts_with('!') {
                bail!("activation sequence cannot start with `!`");
            }
            queries::add_activator(user.as_str(), name, &activator)
                .await
                .context("Error adding activator")?;
            let msg = format!("Added activator `{}` to {}", activator, name);
            room.send(RoomMessageEventContent::text_markdown(msg), None)
                .await
                .context("Error sending reply")?;
        }
        "remove" | "rm" => {
            let activator = cmd.pop_word().ok_or_else(|| {
                anyhow!(
                    "!member [member] activator remove needs the activation sequence as an arquement"
                )
            })?;
            queries::remove_activator(user.as_str(), name, &activator).await?;
            let msg = format!("Removed activator `{}` from {}", activator, name);
            room.send(RoomMessageEventContent::text_markdown(msg), None)
                .await
                .context("Error sending reply")?;
        }
        unknown => bail!("Unkown sub-command `{unknown}`"),
    }
    Ok(())
}

async fn toggle_track_acc(room: &Joined, user: &UserId, name: &str) -> anyhow::Result<()> {
    let msg = if queries::toggle_tracking(user.as_str(), name)
        .await
        .context("Error toggling track account")?
    {
        "enabled"
    } else {
        "disabled"
    };
    let msg = format!("Tracking account {msg} for {name}");
    room.send(RoomMessageEventContent::text_markdown(msg), None)
        .await
        .context("Error sending reply")?;
    Ok(())
}
