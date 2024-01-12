use anyhow::Context;
use chrono::Utc;
use poise::serenity_prelude::User;
use robbb_db::mod_action::ModActionKind;
use serenity::client;

use crate::modlog;

use super::*;

const TIMEOUT_MAX_DAYS: i64 = 28;

#[derive(poise::Modal)]
#[name = "Mute"]
struct MuteModal {
    duration: String,
    #[paragraph]
    reason: Option<String>,
}

#[poise::command(
    guild_only,
    context_menu_command = "Mute",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn menu_mute(app_ctx: AppCtx<'_>, user: User) -> Res<()> {
    let guild = app_ctx.guild().context("Not in a guild")?.to_owned();
    let member = guild.member(&app_ctx.serenity_context(), user.id).await?;

    let response: Option<MuteModal> = poise::execute_modal(app_ctx, None, None).await?;
    if let Some(response) = response {
        let duration =
            response.duration.parse::<humantime::Duration>().user_error("Invalid duration")?;
        do_mute(app_ctx.into(), member.as_ref(), duration, response.reason).await?;
    } else {
        Ctx::Application(app_ctx).say_error("Cancelled").await?;
    }
    Ok(())
}

/// Mute a user for a given amount of time.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Helper }"
)]
pub async fn mute(
    ctx: Ctx<'_>,
    #[description = "User"] user: Member,
    #[description = "Duration of the mute"] duration: humantime::Duration,
    #[description = "Reason"]
    #[rest]
    reason: Option<String>,
) -> Res<()> {
    do_mute(ctx, &user, duration, reason).await?;
    Ok(())
}

/// Run a mute from a command or context menu
async fn do_mute(
    ctx: Ctx<'_>,
    member: &Member,
    duration: humantime::Duration,
    reason: Option<String>,
) -> Res<()> {
    let police = ctx.get_up_emotes().map(|x| x.police.to_string()).unwrap_or_default();
    let success_msg = ctx
        .say(format!(
            "{police}{police} Muting {} for {}. {police}{police}{}",
            member.mention(),
            duration,
            reason.as_ref().map(|x| format!("\nReason: {}", x)).unwrap_or_default()
        ))
        .await?;
    let success_msg = success_msg.message().await?;

    apply_mute(
        ctx.serenity_context(),
        ctx.author().id,
        member.clone(),
        *duration,
        reason.clone(),
        success_msg.link(),
    )
    .await?;

    modlog::log_mute(&ctx, &success_msg, &member.user, duration, reason).await;
    Ok(())
}

/// mute the user and add the mute-entry to the database.
pub async fn apply_mute(
    ctx: &client::Context,
    moderator: UserId,
    mut member: Member,
    duration: std::time::Duration,
    reason: Option<String>,
    context: String,
) -> anyhow::Result<()> {
    let db = ctx.get_db().await;

    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::from_std(duration).unwrap();

    // Ensure only one active mute per member
    db.remove_active_mutes(member.user.id).await?;

    db.add_mod_action(
        moderator,
        member.user.id,
        reason.unwrap_or_else(|| "no reason".to_string()),
        start_time,
        context,
        ModActionKind::Mute { end_time, active: true },
    )
    .await?;

    // TODORW possibly make this actually work for longer timeouts, via re-adding the timeout
    // Also set a discord timeout when possible
    let latest_possible_timeout = Utc::now()
        .checked_add_signed(chrono::Duration::days(TIMEOUT_MAX_DAYS))
        .context("Overflow calculating max date")?
        .date_naive();

    if end_time.date_naive() <= latest_possible_timeout {
        member.disable_communication_until_datetime(&ctx, end_time.into()).await?;
    }

    set_mute_role(ctx, member).await?;
    Ok(())
}

/// Adds the mute role to the user, but does _not_ add any database entry.
/// This should only be used if we know that an active database entry for the mute already exists,
/// or else we run the risk of accidentally muting someone forever.
pub async fn set_mute_role(ctx: &client::Context, member: Member) -> anyhow::Result<()> {
    let config = ctx.get_config().await;
    member.add_role(&ctx, config.role_mute).await?;
    Ok(())
}
