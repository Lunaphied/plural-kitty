mod commands;
mod parser;

use std::{path::Path, sync::atomic::AtomicBool, time::Duration};

use anyhow::{anyhow, Context};
use matrix_sdk::{
    config::SyncSettings,
    room::Room,
    ruma::events::room::member::{OriginalSyncRoomMemberEvent, StrippedRoomMemberEvent},
    Account, Client, Session,
};
use tokio::time::sleep;

use crate::{config::CONFIG, db::queries};

pub static STARTED: AtomicBool = AtomicBool::new(false);

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
    async fn load_prev_login(session_file_path: &Path) -> anyhow::Result<Client> {
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
    }
    async fn new_login(password: &str, session_file_path: &Path) -> anyhow::Result<Client> {
        let client = client().await?;
        let session: Session = client
            .login_username(CONFIG.bot.user.as_str(), password)
            .initial_device_display_name("Plural Relay")
            .send()
            .await
            .context("Error login in")?
            .into();
        let session_json =
            serde_json::to_vec(&session).context("Error serializing session data")?;
        tokio::fs::write(session_file_path, session_json)
            .await
            .with_context(|| {
                anyhow!(
                    "Error writing session data to {}",
                    session_file_path.display()
                )
            })?;
        Ok(client)
    }

    // if password file is set
    //     if session file exist
    //         load session from session file and run
    //     else
    //         login with password file
    // if secret file set and exists
    //     if state store exists
    //         load session from secret file
    //     else
    //         load password from secret file and log in
    //         save session to secret file
    // else
    //     if session file exists
    //         load session from session file and run
    //     else
    //         interactive login

    if let Some(password_file_apth) = &CONFIG.bot.password_file {
        let session_file_path = CONFIG.bot.session_file_path();
        if session_file_path.exists() {
            Ok(load_prev_login(&session_file_path).await?)
        } else {
            let password = tokio::fs::read_to_string(password_file_apth)
                .await
                .with_context(|| {
                    format!(
                        "Error reading password from file {}",
                        password_file_apth.display()
                    )
                })?;
            Ok(new_login(&password, &session_file_path).await?)
        }
    } else if let Some(secret_file_path) = &CONFIG.bot.secret_file {
        if CONFIG.bot.state_store.exists() {
            Ok(load_prev_login(secret_file_path).await?)
        } else {
            let password = tokio::fs::read_to_string(secret_file_path)
                .await
                .with_context(|| {
                    format!(
                        "Error reading password from file {}",
                        secret_file_path.display()
                    )
                })?;
            Ok(new_login(&password, secret_file_path).await?)
        }
    } else {
        let session_file_path = CONFIG.bot.session_file_path();
        if session_file_path.exists() {
            Ok(load_prev_login(&session_file_path).await?)
        } else {
            let password = rpassword::prompt_password("Please entry Plutal Kitty's matrix account password: ")
                .context("Error reading Plural Kitty's password from stdin. If you do not want interactive password login please set either password_file or secret_file in your config")?;
            Ok(new_login(&password, &session_file_path).await?)
        }
    }
}

#[tokio::main]
pub async fn init() -> anyhow::Result<()> {
    let client = create_client().await.context("Error creating bot client")?;

    // Update display name and avatar if needed
    if let Err(e) = update_account_info(&client.account()).await {
        tracing::error!("Error updating bot account info: {e:}");
    }

    // An initial sync to set up state and so our bot doesn't respond to old
    // messages. If the `StateStore` finds saved state in the location given the
    // initial sync will be skipped in favor of loading state from the store
    tokio::spawn(async {
        tracing::info!("Updating tracking members");
        match update_tracking_members().await {
            Ok(err) => {
                for e in err {
                    tracing::error!("{e:#}");
                }
            }
            Err(e) => tracing::error!("Error updating tracking members: {e:#}"),
        }
    });
    let response = client
        .sync_once(SyncSettings::default())
        .await
        .context("Initial sync failed")?;
    tracing::info!("Initial sync done");
    // DM message handler
    client.add_event_handler(commands::dm_handler);
    // Auto join room bot is invited to
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
    client.add_event_handler(
        |event: OriginalSyncRoomMemberEvent, room: Room| async move {
            if let Room::Joined(_) = room {
                tracing::debug!("Profile updated maybe");
                if let Err(e) = update_user_tracking_members(event.sender.as_str()).await {
                    tracing::error!("Error updating info for {}: {e:#}", event.sender);
                }
            }
        },
    );
    let settings = SyncSettings::default().token(response.next_batch);
    STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
    client.sync(settings).await?;

    Ok(())
}

async fn update_tracking_members() -> anyhow::Result<Vec<anyhow::Error>> {
    let mut errs = vec![];
    for user in queries::get_users()
        .await
        .context("Error getting user list")?
    {
        tracing::debug!("Updating tracking for {user}");
        if let Err(e) = update_user_tracking_members(&user)
            .await
            .with_context(|| format!("Error updating tracking members for {user}"))
        {
            errs.push(e);
        }
    }
    Ok(errs)
}

async fn update_user_tracking_members(mxid: &str) -> anyhow::Result<()> {
    let profile = queries::get_synapse_profile(mxid).await?;
    queries::update_tracking_member(mxid, &profile)
        .await
        .with_context(|| format!("Error updating info for {mxid}"))?;
    Ok(())
}

async fn update_account_info(account: &Account) -> anyhow::Result<()> {
    let display_name = account
        .get_display_name()
        .await
        .context("Error getting bot display name")?;
    if CONFIG.bot.display_name.is_some() && CONFIG.bot.display_name != display_name {
        account
            .set_display_name(CONFIG.bot.display_name.as_deref())
            .await
            .context("Error setting bot display name")?;
    }
    let avatar = account
        .get_avatar_url()
        .await
        .context("Error getting bot avatar")?;
    if CONFIG.bot.avatar.is_some() && CONFIG.bot.avatar != avatar {
        account
            .set_avatar_url(CONFIG.bot.avatar.as_deref())
            .await
            .context("Error setting bot avatar")?;
    }
    Ok(())
}
