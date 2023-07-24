#![allow(dead_code)]
mod member;
mod system;

use std::future::Future;

use anyhow::Context;
use matrix_sdk::room::{Joined, Receipts, Room};
use matrix_sdk::ruma::api::client::receipt::create_receipt::v3::ReceiptType as ReactType;
use matrix_sdk::ruma::events::reaction::ReactionEventContent;
use matrix_sdk::ruma::events::receipt::ReceiptThread;
use matrix_sdk::ruma::events::relation::Annotation;
use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};
use matrix_sdk::ruma::OwnedEventId;
use matrix_sdk::Client;

use crate::bot::parser::{Cmd, CmdPart};
use crate::db::queries;

pub type ErrList = Vec<anyhow::Error>;

const HELP: &str = "
# Plural Kitty Help
...
";

pub async fn dm_handler(
    event: OriginalSyncRoomMessageEvent,
    client: Client,
    room: Room,
) -> anyhow::Result<()> {
    if event.sender == client.user_id().unwrap() {
        return Ok(());
    }
    if let Room::Joined(room) = room {
        // Only respond to DMs
        tracing::debug!("Processing event {}", event.event_id);
        if !room.is_direct().await? {
            return Ok(());
        }
        tokio::spawn({
            let room = room.clone();
            let event_id = event.event_id.clone();
            async move {
                let new_receipts = Receipts::new().public_read_receipt(event_id);
                if let Err(e) = room.send_multiple_receipts(new_receipts).await {
                    tracing::error!("Error sending receipt for message: {e:#}",);
                }
            }
        });
        if queries::read(room.room_id().as_str(), event.event_id.as_str()).await? {
            tracing::debug!("Skipping already seen message");
            return Ok(());
        }
        let handler = Handler {
            room: room.clone(),
            cmd_event_id: event.event_id.clone(),
        };
        match &event.content.msgtype {
            MessageType::Text(message_content) => {
                let mut cmd = Cmd::parse(message_content)?;
                tracing::debug!("{cmd:?}");
                if let Some(CmdPart::Word(word)) = cmd.pop() {
                    if word.starts_with('!') {
                        match word.as_str() {
                            "!member" | "!m" => handler.run(member::exec(cmd, &room, &event)).await,
                            "!system" | "!s" => {
                                handler.run(system::exec(&room, &event.sender)).await
                            }
                            "!help" => handler.run_no_feddback(help(cmd, &room)).await,
                            _ => {
                                let content = RoomMessageEventContent::notice_markdown(
                                "Unknown command. Type `!help` for for a list command and what they do.",
                            );
                                room.send(content, None).await?;
                            }
                        }
                    } else if let Some(member_name) =
                        queries::get_name_for_activator(event.sender.as_str(), &word)
                            .await
                            .context("Error getting activator")?
                    {
                        queries::set_current_identity(event.sender.as_str(), &member_name)
                            .await
                            .context("Error setting current member")?;
                        room.send(
                            RoomMessageEventContent::notice_markdown(format!(
                                "Set current fronter to {member_name}"
                            )),
                            None,
                        )
                        .await?;
                    } else {
                        let msg = format!("Unknown command or activator\n\n{HELP}");
                        room.send(RoomMessageEventContent::notice_markdown(msg), None)
                            .await?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Clone)]
struct Handler {
    room: Joined,
    cmd_event_id: OwnedEventId,
}

impl Handler {
    async fn run(self, f: impl Future<Output = anyhow::Result<ErrList>>) {
        let content =
            ReactionEventContent::new(Annotation::new(self.cmd_event_id.clone(), "⏳".to_owned()));
        let reaction_event_id = match self.room.send(content, None).await {
            Ok(resp) => Some(resp.event_id),
            Err(e) => {
                tracing::warn!("Error setting pending reaction: {e:?}");
                None
            }
        };
        let res = f.await;
        if let Some(reaction_event_id) = reaction_event_id.as_ref() {
            if let Err(e) = self.room.redact(reaction_event_id, None, None).await {
                tracing::warn!("Error redaction pending reaction: {e:?}");
            }
        }
        match res {
            Ok(errors) => {
                if errors.is_empty() {
                    let content = ReactionEventContent::new(Annotation::new(
                        self.cmd_event_id,
                        "✅".to_owned(),
                    ));
                    if let Err(e) = self.room.send(content, None).await {
                        tracing::error!("Error sending success reaction: {e}");
                    }
                } else {
                    let content = ReactionEventContent::new(Annotation::new(
                        self.cmd_event_id,
                        "❌".to_owned(),
                    ));
                    if let Err(e) = self.room.send(content, None).await {
                        tracing::error!("Error sending error reaction: {e}");
                    }
                    for e in errors {
                        tracing::error!("Error in command handler: {e:?}");
                        let content = RoomMessageEventContent::notice_plain(format!("{e:?}"));
                        if let Err(e) = self.room.send(content, None).await {
                            tracing::error!("Error sending error message: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error in command handler: {e:?}");
                let content =
                    ReactionEventContent::new(Annotation::new(self.cmd_event_id, "❌".to_owned()));
                if let Err(e) = self.room.send(content, None).await {
                    tracing::error!("Error sending error reaction: {e}");
                }
                let content = RoomMessageEventContent::notice_plain(format!("{e:?}"));
                if let Err(e) = self.room.send(content, None).await {
                    tracing::error!("Error sending error message: {e}");
                }
            }
        }
    }

    async fn run_nore(self, f: impl Future<Output = anyhow::Result<ErrList>>) {
        match f.await {
            Ok(errors) => {
                if !errors.is_empty() {
                    let content = ReactionEventContent::new(Annotation::new(
                        self.cmd_event_id,
                        "❌".to_owned(),
                    ));
                    if let Err(e) = self.room.send(content, None).await {
                        tracing::error!("Error sending error reaction: {e}");
                    }
                    for e in errors {
                        tracing::error!("Error in command handler: {e:?}");
                        let content = RoomMessageEventContent::notice_plain(format!("{e:?}"));
                        if let Err(e) = self.room.send(content, None).await {
                            tracing::error!("Error sending error message: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error in command handler: {e:?}");
                let content = RoomMessageEventContent::notice_plain(format!("{e:?}"));
                if let Err(e) = self.room.send(content, None).await {
                    tracing::error!("Error sending error message: {e}");
                }
            }
        }
    }

    async fn run_no_feddback(self, f: impl Future<Output = anyhow::Result<ErrList>>) {
        if let Err(e) = self
            .room
            .send_single_receipt(
                ReactType::Read,
                ReceiptThread::Unthreaded,
                self.cmd_event_id,
            )
            .await
        {
            tracing::error!("Error posting read  receipt: {e:#}");
        }
        if let Err(e) = f.await {
            tracing::error!("Error in command handler: {e:#}");
        }
    }

    fn cmd_reactor(self) -> CmdRector {
        CmdRector {
            reaction_event_id: None,
            handler: self,
            paused: false,
        }
    }
}

#[derive(Clone)]
pub struct CmdRector {
    reaction_event_id: Option<OwnedEventId>,
    handler: Handler,
    paused: bool,
}

impl CmdRector {
    pub async fn start(&mut self) {
        let content = ReactionEventContent::new(Annotation::new(
            self.handler.cmd_event_id.clone(),
            "⏳".to_owned(),
        ));
        self.reaction_event_id = match self.handler.room.send(content, None).await {
            Ok(resp) => Some(resp.event_id),
            Err(e) => {
                tracing::warn!("Error setting pending reaction: {e:?}");
                None
            }
        };
    }

    pub async fn success(mut self) {
        if let Some(reaction_event_id) = self.reaction_event_id.take() {
            let handler = self.handler.clone();
            tokio::task::spawn(async move {
                if let Err(e) = handler.room.redact(&reaction_event_id, None, None).await {
                    tracing::warn!("Error redaction pending reaction: {e:?}");
                }
                let content = ReactionEventContent::new(Annotation::new(
                    handler.cmd_event_id,
                    "✅".to_owned(),
                ));
                if let Err(e) = handler.room.send(content, None).await {
                    tracing::error!("Error sending error reaction: {e}");
                }
            });
        }
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }
}

impl Drop for CmdRector {
    fn drop(&mut self) {
        if self.paused {
            return;
        }
        if let Some(reaction_event_id) = self.reaction_event_id.take() {
            let handler = self.handler.clone();
            tokio::task::spawn(async move {
                if let Err(e) = handler.room.redact(&reaction_event_id, None, None).await {
                    tracing::warn!("Error redaction pending reaction: {e:?}");
                }
                let content = ReactionEventContent::new(Annotation::new(
                    handler.cmd_event_id,
                    "❌".to_owned(),
                ));
                if let Err(e) = handler.room.send(content, None).await {
                    tracing::error!("Error sending error reaction: {e}");
                }
            });
        }
    }
}

pub async fn help(mut cmd: Cmd, room: &Joined) -> anyhow::Result<ErrList> {
    let word = cmd.pop_word();
    let message = match word.as_deref() {
        _ => HELP,
    };
    let content = RoomMessageEventContent::notice_markdown(message);
    room.send(content, None).await?;
    Ok(vec![])
}
