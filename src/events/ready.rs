use serenity::futures::StreamExt;

use super::*;

pub async fn ready(ctx: client::Context, _data_about_bot: Ready) -> Result<()> {
    let bot_version = util::bot_version();
    tracing::info!("Robbb is ready! Running version {}", &bot_version);

    let config = ctx.get_config().await;

    match load_up_emotes(&ctx, config.guild).await {
        Ok(emotes) => {
            ctx.data.write().await.insert::<UpEmotes>(Arc::new(emotes));
        }
        Err(err) => {
            tracing::warn!("Error loading emotes: {}", err);
        }
    }

    let _ = config
        .channel_mod_bot_stuff
        .send_embed(&ctx, |e| {
            e.title("Bot is back!");
            e.description("Hey guys, I'm back!");
            e.field("version", bot_version, false);
        })
        .await;

    let _ = ctx
        .set_presence(Some(Activity::listening("!help")), OnlineStatus::Online)
        .await;

    dehoist_everyone(ctx.clone(), config.guild).await;

    start_mute_handler(ctx.clone()).await;
    start_attachment_log_handler(ctx).await;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn dehoist_everyone(ctx: client::Context, guild_id: GuildId) {
    guild_id
        .members_iter(&ctx)
        .filter_map(|x| async { x.ok() })
        .for_each_concurrent(None, |member| async {
            log_error!(
                "Error while dehoisting a member",
                guild_member_update::dehoist_member(ctx.clone(), member).await
            );
        })
        .await;
}

async fn start_mute_handler(ctx: client::Context) {
    tokio::spawn(async move {
        let _ =
            tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None);
        let (config, db) = ctx.get_config_and_db().await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let mutes = match db.get_newly_expired_mutes().await {
                Ok(mutes) => mutes,
                Err(err) => {
                    tracing::error!(error.message = %err, "Failed to request expired mutes: {}", err);
                    continue;
                }
            };
            for mute in mutes {
                if let Err(err) = unmute(&ctx, &config, &db, &mute).await {
                    tracing::error!(error.message = %err, "Error handling mute removal: {}", err);
                } else {
                    config
                        .log_bot_action(&ctx, |e| {
                            e.description(format!("{} is now unmuted", mute.user.mention()));
                        })
                        .await;
                }
            }
        }
    });
}

async fn start_attachment_log_handler(ctx: client::Context) {
    tokio::spawn(async move {
        let _ =
            tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None);
        let config = ctx.get_config().await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            log_error!(
                "Failed to clean up attachments",
                crate::attachment_logging::cleanup(&config).await
            );
        }
    });
}

async fn load_up_emotes(ctx: &client::Context, guild: GuildId) -> Result<UpEmotes> {
    let all_emoji = guild.emojis(&ctx).await?;
    Ok(UpEmotes {
        pensibe: all_emoji
            .iter()
            .find(|x| x.name == "pensibe")
            .context("no pensibe emote found")?
            .clone(),
        police: all_emoji
            .iter()
            .find(|x| x.name == "police")
            .context("no police emote found")?
            .clone(),
        poggers: all_emoji
            .iter()
            .find(|x| x.name == "poggersphisch")
            .context("no police poggers found")?
            .clone(),
        stares: all_emoji
            .into_iter()
            .filter(|x| x.name.starts_with("stare"))
            .collect(),
    })
}
