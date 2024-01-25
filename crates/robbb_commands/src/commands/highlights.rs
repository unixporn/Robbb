use poise::serenity_prelude::CreateEmbed;

use super::*;
use crate::checks::{self, PermissionLevel};

/// Get notified when someone mentions a word you care about.
#[poise::command(
    slash_command,
    rename = "highlight",
    aliases("highlights", "hl"),
    subcommands("highlights_add", "highlights_list", "highlights_clear", "highlights_remove",)
)]
pub async fn highlights(_: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Add a new highlight
#[poise::command(slash_command, guild_only, rename = "add")]
pub async fn highlights_add(
    ctx: Ctx<'_>,
    #[description = "The word you want to be notified about"] trigger: String,
) -> Res<()> {
    ctx.defer().await?;
    if trigger.len() < 3 {
        abort_with!("Highlight has to be longer than 2 characters");
    }

    let db = ctx.get_db();
    let max_highlight_cnt =
        match checks::get_permission_level(ctx.serenity_context(), ctx.author()).await? {
            PermissionLevel::Mod => 20,
            _ => 4,
        };

    let highlights = db.get_highlights().await?;
    let highlights_by_user_cnt = highlights.triggers_for_user(ctx.author().id).count();

    if highlights_by_user_cnt >= max_highlight_cnt {
        abort_with!(UserErr::new(format!(
            "Sorry, you can only watch a maximum of {} highlights",
            max_highlight_cnt
        )));
    }

    ctx.author()
        .id
        .create_dm_channel(&ctx.serenity_context())
        .await
        .user_error("Couldn't open a DM to you - do you have me blocked?")?
        .send_message(
            &ctx.serenity_context(),
            CreateEmbed::default()
                .title("Test to see if you can receive DMs")
                .description(format!(
                    "If everything went ok, you'll be notified whenever someone says `{trigger}`",
                ))
                .into_create_message(),
        )
        .await
        .user_error("Couldn't send you a DM :/\nDo you allow DMs from server members?")?;

    db.set_highlight(ctx.author().id, trigger.clone()).await.user_error(
        "Couldn't add highlight, something went wrong (highlight might already be present)",
    )?;

    ctx.say_success(format!("You will be notified whenever someone says {trigger}")).await?;

    Ok(())
}

/// List all of your highlights
#[poise::command(slash_command, guild_only, rename = "list")]
pub async fn highlights_list(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();
    let highlights = db.get_highlights().await?;

    let highlights_list = highlights.triggers_for_user(ctx.author().id).join("\n");

    if highlights_list.is_empty() {
        abort_with!("You don't seem to have set any highlights");
    } else {
        ctx.reply_embed_ephemeral_builder(|e| {
            e.title("Your highlights").description(highlights_list)
        })
        .await?;
    }
    Ok(())
}

/// Remove a highlight
#[poise::command(slash_command, guild_only, rename = "remove")]
pub async fn highlights_remove(
    ctx: Ctx<'_>,
    #[autocomplete = "autocomplete_highlights"]
    #[description = "Which highlight do you want to remove"]
    trigger: String,
) -> Res<()> {
    let db = ctx.get_db();
    db.remove_highlight(ctx.author().id, trigger.clone())
        .await
        .user_error("Failed to remove the highlight.")?;
    ctx.say_success(format!("You will no longer be notified when someone says '{}'", trigger))
        .await?;
    Ok(())
}

/// Remove all of your highlights
#[poise::command(slash_command, guild_only, rename = "clear")]
pub async fn highlights_clear(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();
    db.rm_highlights_of(ctx.author().id).await?;
    ctx.say_success("Your highlights have been successfully cleared.").await?;
    Ok(())
}

async fn autocomplete_highlights(ctx: Ctx<'_>, partial: &str) -> Vec<String> {
    let db = ctx.get_db();
    if let Ok(highlights) = db.get_highlights().await {
        highlights
            .triggers_for_user(ctx.author().id)
            .filter(|x| x.contains(partial))
            .map(|x| x.to_string())
            .collect_vec()
    } else {
        Vec::new()
    }
}
