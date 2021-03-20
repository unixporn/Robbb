use super::*;

/// Restart the bot.
#[command]
#[usage("restart")]
pub async fn restart(ctx: &client::Context, msg: &Message) -> CommandResult {
    let _ = msg.reply(&ctx, "Shutting down").await;
    ctx.shard.shutdown_clean();

    std::process::exit(1);
}

/// Make the bot say something. Please don't actually use this :/
#[command]
#[usage("say <something>")]
pub async fn say(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let content = args.remains().invalid_usage(&SAY_COMMAND_OPTIONS)?;
    msg.channel_id
        .send_message(&ctx, |m| m.content(content))
        .await?;
    msg.delete(&ctx).await?;
    Ok(())
}

/// Print bot's latency to discord.
#[command]
#[usage("latency")]
pub async fn latency(ctx: &client::Context, msg: &Message) -> CommandResult {
    let msg_time = msg.timestamp;
    let now = Utc::now();
    let latency = now.timestamp_millis() - msg_time.timestamp_millis();
    msg.reply(&ctx, format!("Latency is **{}ms**", latency))
        .await?;

    Ok(())
}

/// Sends a link to the bot's repository! Feel free contribute!
#[command]
#[usage("repo")]
pub async fn repo(ctx: &client::Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx, "https://github.com/unixporn/trup-rs")
        .await?;
    Ok(())
}

/// set your profiles description.
#[command]
#[usage("desc <text>")]
pub async fn desc(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<Db>().unwrap().clone();

    let value = args.remains().map(|x| x.to_string());
    db.set_description(msg.author.id, value).await?;

    msg.reply_success(&ctx, "Successfully updated your description!")
        .await?;
    Ok(())
}

/// Provide a link to your git.
#[command]
#[usage("git <url>")]
pub async fn git(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<Db>().unwrap().clone();

    let value = args.remains().map(|x| x.to_string());
    if value.as_ref().map(|x| util::validate_url(&x)) == Some(false) {
        abort_with!(UserErr::other("Malformed URL"));
    }
    db.set_git(msg.author.id, value).await?;

    msg.reply_success(&ctx, "Successfully updated your git-url!")
        .await?;
    Ok(())
}

/// Provide a link to your dotfiles
#[command]
#[usage("dotfiles <url>")]
pub async fn dotfiles(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<Db>().unwrap().clone();

    let value = args.remains().map(|x| x.to_string());
    if value.as_ref().map(|x| util::validate_url(&x)) == Some(false) {
        abort_with!(UserErr::other("Malformed URL"));
    }
    db.set_dotfiles(msg.author.id, value).await?;

    msg.reply_success(&ctx, "Successfully updated your dotfiles!")
        .await?;
    Ok(())
}

#[command]
#[usage("invite")]
pub async fn invite(ctx: &client::Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx, "https://discord.gg/TnJ4h5K").await?;
    Ok(())
}
