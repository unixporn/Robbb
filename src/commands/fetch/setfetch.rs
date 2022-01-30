use super::*;
use crate::extensions::StrExt;
use std::collections::HashMap;
use std::str::FromStr;

const SETFETCH_USAGE: &str = indoc::indoc!("
    Run this: 
    `curl -s https://raw.githubusercontent.com/unixporn/robbb/master/fetcher.sh | sh`
    and follow the instructions. It's recommended that you download and read the script before running it, 
    as piping curl to sh isn't always the safest practice. (<https://blog.dijit.sh/don-t-pipe-curl-to-bash>) 

    **NOTE**: use `!setfetch update` to update individual values (including the image!) without overwriting everything.
    **NOTE**: !git, !dotfiles, and !desc are different commands"
);

/// Run without arguments to see instructions.
#[command("setfetch")]
#[usage("setfetch [update | clear]")]
#[sub_commands(set_fetch_update, set_fetch_clear)]
pub async fn set_fetch(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let lines = args.rest().lines().collect_vec();
    do_set_fetch(ctx, msg, lines, false).await
}

#[command("update")]
#[usage("setfetch update")]
pub async fn set_fetch_update(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let lines = args.rest().lines().collect_vec();
    do_set_fetch(ctx, msg, lines, true).await
}

#[command("clear")]
#[usage("setfetch clear")]
pub async fn set_fetch_clear(ctx: &client::Context, msg: &Message) -> CommandResult {
    let db = ctx.get_db().await;
    db.set_fetch(msg.author.id, HashMap::new(), Some(Utc::now()))
        .await?;
    msg.reply_success(&ctx, "Successfully cleared your fetch data!")
        .await?;
    Ok(())
}

async fn do_set_fetch(
    ctx: &client::Context,
    msg: &Message,
    lines: Vec<&str>,
    update: bool,
) -> CommandResult {
    let db = ctx.get_db().await;

    if lines.is_empty() && msg.attachments.is_empty() {
        msg.reply_embed(&ctx, |e| {
            e.title("Usage").description(SETFETCH_USAGE);
        })
        .await?;
        return Ok(());
    }

    let mut info = sanitize_fetch(
        parse_setfetch(lines).user_error("Illegal format, please use `field: value` syntax.")?,
    )?;

    let image_url: Option<String> = msg.find_image_urls().first().cloned();

    if let Some(image) = image_url {
        info.insert(FetchField::Image, image);
    }

    if update {
        db.update_fetch(msg.author.id, info).await?;
        msg.reply_success(&ctx, "Successfully updated your fetch data!")
            .await?;
    } else {
        db.set_fetch(msg.author.id, info, Some(Utc::now())).await?;
        msg.reply_success(&ctx, "Successfully set your fetch data!")
            .await?;
    }

    Ok(())
}

/// parse key:value formatted lines into a hashmap.
fn parse_setfetch(lines: Vec<&str>) -> Result<HashMap<String, String>> {
    lines
        .into_iter()
        .map(|line| {
            line.split_once_at(':')
                .map(|(l, r)| (l.trim().to_string(), r.trim().to_string()))
                .filter(|(k, _)| !k.is_empty())
                .context("Malformed line")
        })
        .collect::<Result<HashMap<String, String>>>()
}

/// Sanitize field values and check validity of user-provided fetch data.
fn sanitize_fetch(fetch: HashMap<String, String>) -> Result<HashMap<FetchField, String>, UserErr> {
    let mut new: HashMap<FetchField, String> = HashMap::new();
    for (key, value) in fetch.into_iter() {
        let field = FetchField::from_str(&key)
            .map_err(|_| UserErr::Other(format!("Illegal fetch field: {}", key)))?;
        let value = match field {
            FetchField::Memory => byte_unit::Byte::from_str(&value)
                .user_error("Malformed value provided for Memory")?
                .get_bytes()
                .to_string(),

            FetchField::Image if !util::validate_url(&value) => {
                abort_with!("Malformed url provided for Image")
            }
            _ => value,
        };
        new.insert(field, value.to_string());
    }
    Ok(new)
}
