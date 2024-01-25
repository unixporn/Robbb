use poise::serenity_prelude::MessageUpdateEvent;

use super::*;

pub async fn message_update(
    ctx: &client::Context,
    old_if_available: Option<Message>,
    new: Option<Message>,
    event: MessageUpdateEvent,
) -> Result<()> {
    let config = ctx.get_config().await;

    if Some(config.guild) != event.guild_id
        || event.edited_timestamp.is_none()
        || event.author.as_ref().map(|x| x.bot).unwrap_or(false)
    {
        return Ok(());
    };

    tracing::info!(
        msg.id = %event.id,
        msg.content = new.map(|x| x.content).unwrap_or_default(),
        "handling message_update event"
    );

    let mut msg = event.channel_id.message(&ctx, event.id).await?;
    msg.guild_id = event.guild_id;

    match handle_blocklist::handle_blocklist(&ctx, &msg).await {
        Ok(false) => {}
        err => log_error!("error while handling blocklist in message_update", err),
    };

    let channel_name =
        util::channel_name(&ctx, event.channel_id).await.unwrap_or_else(|_| "unknown".to_string());

    config
        .guild
        .send_embed(&ctx, config.channel_bot_messages, |mut e| {
            if let Some(edited_timestamp) = event.edited_timestamp {
                e = e.timestamp(edited_timestamp);
            }
            e.author_icon("Message Edit", msg.author.face())
                .title(msg.author.name_with_disc_and_id())
                .description(indoc::formatdoc!(
                    "
                        **Before:**
                        {}

                        **Now:**
                        {}

                        {}
                    ",
                    old_if_available
                        .map(|old| old.content)
                        .unwrap_or_else(|| "<Unavailable>".to_string()),
                    event.content.clone().unwrap_or_else(|| "<Unavailable>".to_string()),
                    msg.to_context_link()
                ))
                .footer_str(format!("#{channel_name}"))
        })
        .await?;
    Ok(())
}
