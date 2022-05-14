use itertools::Itertools;
use poise::serenity_prelude::Member;
use poise::serenity_prelude::{Guild, Mentionable, UserId};
use poise::Command;
use robbb_util::abort_with;
use robbb_util::extensions::*;
use robbb_util::prelude::*;
use robbb_util::util;

pub mod errors;
pub use errors::*;

pub mod ask;
pub mod ban;
pub mod blocklist;
pub mod emojistats;
pub mod fetch;
pub mod help;
pub mod highlights;
pub mod info;
pub mod kick;
pub mod modping;
pub mod move_users;
pub mod mute;
pub mod note;
pub mod pfp;
pub mod poise_commands;
pub mod poll;
pub mod purge;
pub mod role;
pub mod small;
pub mod tag;
pub mod top;
pub mod unban;
pub mod version;
pub mod warn;

pub fn all_commands() -> Vec<poise::Command<UserData, Error>> {
    vec![
        pfp::pfp(),
        info::info(),
        help::help(),
        role::role(),
        version::version(),
        poll::poll(),
        tag::tag(),
        ask::ask(),
        modping::modping(),
        highlights::highlights(),
        small::latency(),
        small::uptime(),
        small::repo(),
        small::invite(),
        small::desc(),
        small::git(),
        small::dotfiles(),
        fetch::fetch(),
        fetch::set_fetch(),
        top::top(),
        mod_command(),
    ]
}

/// Moderation-related commands
#[poise::command(
    rename = "mod",
    slash_command,
    default_member_permissions = "BAN_MEMBERS",
    subcommands(
        "small::restart",
        "small::say",
        "warn::warn",
        "ban::ban",
        "kick::kick",
        "unban::unban",
        "emojistats::emojistats",
        "blocklist::blocklist",
        "note::note",
        "mute::mute",
        "purge::purge",
        "poise_commands::register",
        "poise_commands::delete",
    )
)]
async fn mod_command(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

pub static SELECTION_EMOJI: [&str; 19] = [
    "1️⃣",
    "2️⃣",
    "3️⃣",
    "4️⃣",
    "5️⃣",
    "6️⃣",
    "7️⃣",
    "8️⃣",
    "9️⃣",
    "🔟",
    "\u{1f1e6}",
    "\u{1f1e7}",
    "\u{1f1e8}",
    "\u{1f1e9}",
    "\u{1f1f0}",
    "\u{1f1f1}",
    "\u{1f1f2}",
    "\u{1f1f3}",
    "\u{1f1f4}",
];

pub async fn member_or_self(ctx: Ctx<'_>, member: Option<Member>) -> Res<Member> {
    if let Some(member) = member {
        Ok(member)
    } else {
        Ok(ctx
            .author_member()
            .await
            .user_error("failed to fetch message author")?)
    }
}
