#![allow(dead_code)]
mod member;
mod system;
mod clear;

use std::future::Future;

use anyhow::Context;
use matrix_sdk::room::{Joined, Receipts, Room};
use matrix_sdk::ruma::events::reaction::ReactionEventContent;
use matrix_sdk::ruma::events::relation::Annotation;
use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};
use matrix_sdk::ruma::OwnedEventId;
use matrix_sdk::{Client, RoomMemberships};

use crate::bot::parser::{Cmd, CmdPart};
use crate::db::queries;

pub type ErrList = Vec<anyhow::Error>;

const HELP: &str = r#"
# Plural Kitty Help

Plural Kitty is a tool that allows users to manage and switch plural identies on Matrix,
similar to Plural Kit for Discord.

- To start using Plural Kitty create a system member: `!member new [member name]`
- You can set a display name like this: `!member [member name] displayname [name]` 
or `!m [member name] dn [display name]`.
- You can set an avatar like this: send `!member [member name] avatar [name]` 
or `!m [member name] av [display name]` in response to the avatar image.
- `!system` or `!s` to view a list of system members.
- You can set clear a display name like this: `!member [member name] displayname !clear` 
or `!m [member name] dn [display name]`.
- You can clear an avatar like this: send `!member [member name] avatar !clear` 
or `!m [member name] av !clear`.
- You can remove a member like this; `!member [member name] remove`.
- To show this help message type `!help`

To switch to a member you must also set one or more activators for that member. Activators are
short text strings that you can type in this DM to switch to the corresponding member.

- You can add an activator like this: `!member [member name] activator add [activator string]` or
`!m [name] act add [activator string]`
- You can remove an activator like this: `!member [member name] activator remove [activator string]` or
`!m [name] act rm [activator string]`

## Example of setting up a member:

```
!m new sasha
!m sasha dn Sashanoraa (ze/zir)
<post an image in this DM>
!m sasha av     (this mesage is in reply to the image posted above)
!m sasha act add s
s               (switch to member sasha)
```
"#;

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
        let members = room.members(RoomMemberships::JOIN).await?;
        if members.len() != 2 {
            tracing::debug!("Ignoring non-DM message");
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
        if queries::read_msgs(room.room_id().as_str(), event.event_id.as_str()).await? {
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
                            "!clear" => handler.run(clear::exec(&room, &event)).await,
                            "!help" => handler.run_no_feddback(help(cmd, &room)).await,
                            _ => {
                                let content = RoomMessageEventContent::text_markdown(
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
                        queries::set_current_identity(event.sender.as_str(), Some(&member_name))
                            .await
                            .context("Error setting current member")?;
                        room.send(
                            RoomMessageEventContent::text_markdown(format!(
                                "Set current fronter to {member_name}"
                            )),
                            None,
                        )
                        .await?;
                    } else {
                        let msg = format!("Unknown command or activator\n\n{HELP}");
                        room.send(RoomMessageEventContent::text_markdown(msg), None)
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
                        let content = RoomMessageEventContent::text_plain(format!("{e:?}"));
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
                let content = RoomMessageEventContent::text_plain(format!("{e:?}"));
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
                        let content = RoomMessageEventContent::text_plain(format!("{e:?}"));
                        if let Err(e) = self.room.send(content, None).await {
                            tracing::error!("Error sending error message: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error in command handler: {e:?}");
                let content = RoomMessageEventContent::text_plain(format!("{e:?}"));
                if let Err(e) = self.room.send(content, None).await {
                    tracing::error!("Error sending error message: {e}");
                }
            }
        }
    }

    async fn run_no_feddback(self, f: impl Future<Output = anyhow::Result<ErrList>>) {
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
    let content = RoomMessageEventContent::text_markdown(message);
    room.send(content, None).await?;
    Ok(vec![])
}
