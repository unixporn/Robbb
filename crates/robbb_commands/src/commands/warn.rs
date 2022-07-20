use chrono::Utc;
use poise::serenity_prelude::User;
use robbb_db::mod_action::{ModActionKind, ModActionType};
use robbb_util::modal::create_modal_command_ir;

use crate::modlog;

use super::*;

#[derive(poise::Modal)]
#[name = "Warn"]
struct WarnModal {
    #[paragraph]
    reason: String,
}

#[poise::command(
    guild_only,
    context_menu_command = "Warn",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn menu_warn(app_ctx: AppCtx<'_>, user: User) -> Res<()> {
    let ctx = Ctx::Application(app_ctx);
    let interaction = match app_ctx.interaction {
        poise::ApplicationCommandOrAutocompleteInteraction::ApplicationCommand(x) => x,
        _ => anyhow::bail!("Menu interaction was not an application command?"),
    };
    let response = create_modal_command_ir::<WarnModal>(app_ctx, interaction, None).await?;
    do_warn(ctx, user, response.reason).await?;
    Ok(())
}

/// Warn a user
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn warn(
    ctx: Ctx<'_>,
    #[description = "Who is the criminal?"]
    #[rename = "criminal"]
    user: User,
    #[description = "What did they do?"]
    #[rest]
    reason: String,
) -> Res<()> {
    do_warn(ctx, user, reason).await?;
    Ok(())
}

async fn do_warn(ctx: Ctx<'_>, user: User, reason: String) -> Res<()> {
    let db = ctx.get_db();
    let warn_count = db.count_mod_actions(user.id, ModActionType::Warn).await?;

    let police = ctx.get_up_emotes().map(|x| x.police.to_string()).unwrap_or_default();

    let success_msg = ctx
        .say(format!(
            "{police}{police} Warning {} for the {} time. {police}{police}\nReason: {}",
            user.mention(),
            util::format_count(warn_count + 1),
            reason,
        ))
        .await?;
    let success_msg = success_msg.message().await?;

    db.add_mod_action(
        ctx.author().id,
        user.id,
        reason.to_string(),
        Utc::now(),
        success_msg.link(),
        ModActionKind::Warn,
    )
    .await?;

    modlog::log_warn(&ctx, &success_msg, user, warn_count + 1, &reason).await;
    Ok(())
}
