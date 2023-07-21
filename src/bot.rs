mod commands;
mod parser;

use std::time::Duration;

use anyhow::{anyhow, Context};
use matrix_sdk::{
    config::SyncSettings, room::Room, ruma::events::room::member::StrippedRoomMemberEvent, Client,
    Session,
};
use tokio::time::sleep;

use crate::config::CONFIG;

pub async fn create_client() -> anyhow::Result<Client> {
    async fn client() -> anyhow::Result<Client> {
        Client::builder()
            .homeserver_url(CONFIG.bot.homeserver_url())
            .respect_login_well_known(true)
            .sqlite_store(&CONFIG.bot.state_store, None)
            .build()
            .await
            .context("Error setting up client")
    }

    let session_file_path = CONFIG.bot.session_file_path();
    if session_file_path.exists() {
        let session_json = tokio::fs::read(&session_file_path).await.with_context(|| {
            anyhow!("Error reading session file {}", session_file_path.display())
        })?;
        let session = serde_json::from_slice(&session_json).with_context(|| {
            anyhow!(
                "Session file {} is not a valid session object",
                session_file_path.display()
            )
        })?;
        let client = client().await?;
        client
            .restore_session(session)
            .await
            .context("Error logging in")?;
        Ok(client)
    } else {
        tracing::info!(
            "Session file {} does not exist, prompting for password...",
            session_file_path.display()
        );
        let pass = rpassword::prompt_password("Please entry Emily's matrix account password: ")
            .context("Error reading Emily's password")?;
        let client = client().await?;
        let session: Session = client
            .login_username(CONFIG.bot.user.as_str(), &pass)
            .initial_device_display_name("Plural Relay")
            .send()
            .await
            .context("Error login in")?
            .into();
        let session_json =
            serde_json::to_vec(&session).context("Error serializing session data")?;
        tokio::fs::write(&session_file_path, session_json)
            .await
            .with_context(|| {
                anyhow!(
                    "Error writing session data to {}",
                    session_file_path.display()
                )
            })?;
        Ok(client)
    }
}

pub async fn init(client: Client) -> anyhow::Result<()> {
    // Log in to matrix

    // An initial sync to set up state and so our bot doesn't respond to old
    // messages. If the `StateStore` finds saved state in the location given the
    // initial sync will be skipped in favor of loading state from the store
    let response = client
        .sync_once(SyncSettings::default())
        .await
        .context("Initial sync failed")?;
    client.add_event_handler(commands::dm_handler);
    client.add_event_handler(
        |room_member: StrippedRoomMemberEvent, client: Client, room: Room| async move {
            if room_member.state_key != client.user_id().unwrap() {
                return;
            }

            if let Room::Invited(room) = room {
                tokio::spawn(async move {
                    println!("Autojoining room {}", room.room_id());
                    let mut delay = 2;

                    while let Err(err) = room.accept_invitation().await {
                        // retry autojoin due to synapse sending invites, before the
                        // invited user can join for more information see
                        // https://github.com/matrix-org/synapse/issues/4345
                        eprintln!(
                            "Failed to join room {} ({err:?}), retrying in {delay}s",
                            room.room_id()
                        );

                        sleep(Duration::from_secs(delay)).await;
                        delay *= 2;

                        if delay > 3600 {
                            eprintln!("Can't join room {} ({err:?})", room.room_id());
                            break;
                        }
                    }
                    println!("Successfully joined room {}", room.room_id());
                });
            }
        },
    );
    tracing::info!("Initial sync done");
    let settings = SyncSettings::default().token(response.next_batch);
    client.sync(settings).await?;

    Ok(())
}
