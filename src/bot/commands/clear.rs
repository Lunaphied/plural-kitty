use matrix_sdk::ruma::events::room::message::{
    OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};
use matrix_sdk::Room;

use crate::db::queries;

use super::ErrList;

pub async fn exec(room: &Room, event: &OriginalSyncRoomMessageEvent) -> anyhow::Result<ErrList> {
    queries::set_current_fronter(event.sender.as_str(), None).await?;
    room.send(RoomMessageEventContent::text_markdown(
        "**Cleared current fronter**",
    ))
    .await?;
    Ok(vec![])
}
