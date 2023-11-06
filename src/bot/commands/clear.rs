use matrix_sdk::room::Joined;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;

use crate::db::queries;

use super::ErrList;

pub async fn exec(room: &Joined, event: &OriginalSyncRoomMessageEvent) -> anyhow::Result<ErrList> {
    queries::set_current_fronter(event.sender.as_str(), None).await?;
    room.send(
        RoomMessageEventContent::text_markdown("**Cleared current fronter**"),
        None,
    )
    .await?;
    Ok(vec![])
}
