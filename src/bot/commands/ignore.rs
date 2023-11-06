use matrix_sdk::room::Joined;
use matrix_sdk::ruma::events::room::message::{
    OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};
use matrix_sdk::ruma::{RoomId, UserId};
use matrix_sdk::Client;

use crate::bot::parser::Cmd;
use crate::db::queries;

use super::ErrList;

pub async fn exec(
    mut cmd: Cmd,
    room: &Joined,
    client: &Client,
    event: &OriginalSyncRoomMessageEvent,
) -> anyhow::Result<ErrList> {
    match cmd.pop_room_id(client).await? {
        Some(room_id) => toggle_ignored(&room_id, &event.sender, room).await?,
        None => list_ignored(&event.sender, room).await?,
    }
    Ok(vec![])
}

async fn list_ignored(user_id: &UserId, room: &Joined) -> anyhow::Result<()> {
    let ignored_rooms = queries::list_ignored(user_id.as_str()).await?;
    if ignored_rooms.is_empty() {
        room.send(
            RoomMessageEventContent::text_plain("#### No ignored rooms"),
            None,
        )
        .await?;
    } else {
        let mut msg = "#### Ignored Rooms\n".to_owned();
        for room_id in ignored_rooms {
            let room_name = queries::room_alias(&room_id)
                .await
                .unwrap_or_else(|_| room_id.to_owned());
            msg += &format!("- {room_name}\n");
        }
        room.send(RoomMessageEventContent::text_markdown(msg), None)
            .await?;
    }
    Ok(())
}

async fn toggle_ignored(room_id: &RoomId, user_id: &UserId, room: &Joined) -> anyhow::Result<()> {
    let currently_ignored = queries::is_room_ignored(user_id.as_str(), room_id.as_str()).await?;
    let room_name = queries::room_alias(room_id.as_str())
        .await
        .unwrap_or_else(|e| {
            tracing::debug!("Error looking up alias {e:#}");
            room_id.as_str().to_owned()
        });
    if currently_ignored {
        queries::unignore_room(user_id.as_str(), room_id.as_str()).await?;
        room.send(
            RoomMessageEventContent::text_markdown(format!("No longer ignoring room {room_name}")),
            None,
        )
        .await?;
    } else {
        queries::ignore_room(user_id.as_str(), room_id.as_str()).await?;
        room.send(
            RoomMessageEventContent::text_markdown(format!("Ignoring room {room_name}")),
            None,
        )
        .await?;
    }
    Ok(())
}
