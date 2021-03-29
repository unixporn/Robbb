use super::*;
use regex::Regex;

lazy_static::lazy_static! {
    static ref POLL_OPTION_START_OF_LINE_PATTERN: Regex = Regex::new(r#"\s*-|^\s*\d\.|^\s*\*"#).unwrap();
}

/// Get people to vote on your question
#[command]
#[usage("poll <question> OR poll multi [title] <one option per line>")]
#[sub_commands(poll_multi)]
pub async fn poll(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let question = args.remains().invalid_usage(&POLL_COMMAND_OPTIONS)?;

    if question.len() > 255 {
        abort_with!("The question is too long :(")
    }

    msg.delete(&ctx).await?;

    msg.channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title("Poll");
                e.description(question);
                e.footer(|f| f.text(format!("From: {}", msg.author.tag())))
            });
            m.reactions(vec![
                ReactionType::Unicode("✅".to_string()),
                ReactionType::Unicode("🤷".to_string()),
                ReactionType::Unicode("❎".to_string()),
            ])
        })
        .await?;
    Ok(())
}

/// Have others select one of many options.
#[command("multi")]
#[usage("poll multi [title] <one option per line>")]
async fn poll_multi(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let mut lines = args.rest().lines().collect_vec();
    let title = lines.first().and_then(|line| line.strip_prefix("multi "));
    if !lines.is_empty() {
        lines.remove(0);
    }

    if lines.len() > SELECTION_EMOJI.len() || lines.len() < 2 {
        abort_with!(UserErr::Other(format!(
            "There must be between 2 and {} options",
            SELECTION_EMOJI.len()
        )))
    }

    msg.delete(&ctx).await?;

    let lines = lines.into_iter().map(|line| {
        POLL_OPTION_START_OF_LINE_PATTERN
            .replace(line, "")
            .to_string()
    });

    let options = SELECTION_EMOJI.iter().zip(lines).collect_vec();

    msg.channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| {
                match title {
                    Some(title) => e.title(format!("Poll: {}", title)),
                    None => e.title("Poll"),
                };
                for (emoji, option) in options.iter() {
                    e.field(format!("Option {}", emoji), option, false);
                }
                e.footer(|f| f.text(format!("from: {}", msg.author.tag())))
            });
            m.reactions(
                options
                    .into_iter()
                    .map(|(emoji, _)| ReactionType::Unicode(emoji.to_string()))
                    .chain(std::iter::once(ReactionType::Unicode("🤷".to_string()))),
            )
        })
        .await?;
    Ok(())
}
