#![allow(clippy::needless_borrow)]
use crate::extensions::*;
use db::Db;
use rand::prelude::IteratorRandom;
use serenity::client::{self, Client};
use serenity::framework::standard::DispatchError;
use serenity::framework::standard::{macros::hook, CommandResult, Reason};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{builder::CreateEmbed, framework::standard::StandardFramework};
use std::{path::PathBuf, sync::Arc};
use tracing::Level;
use tracing_futures::Instrument;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::EnvFilter;

use crate::util::*;
use anyhow::Result;

pub mod attachment_logging;
pub mod checks;
pub mod commands;
pub mod db;
pub mod embeds;
pub mod events;
pub mod extensions;
pub mod util;

use commands::*;

#[derive(Debug, Clone)]
pub struct UpEmotes {
    pensibe: Emoji,
    police: Emoji,
    poggers: Emoji,
    stares: Vec<Emoji>,
}
impl UpEmotes {
    pub fn random_stare(&self) -> Option<Emoji> {
        let mut rng = rand::thread_rng();
        self.stares.iter().choose(&mut rng).cloned()
    }
}

impl TypeMapKey for UpEmotes {
    type Value = Arc<UpEmotes>;
}

pub struct Config {
    pub discord_token: String,

    pub guild: GuildId,
    pub role_mod: RoleId,
    pub role_helper: RoleId,
    pub role_mute: RoleId,
    pub roles_color: Vec<RoleId>,

    pub category_mod_private: ChannelId,
    pub channel_showcase: ChannelId,
    pub channel_feedback: ChannelId,
    pub channel_modlog: ChannelId,
    pub channel_mod_bot_stuff: ChannelId,
    pub channel_auto_mod: ChannelId,
    pub channel_bot_messages: ChannelId,
    pub channel_bot_traffic: ChannelId,
    pub channel_tech_support: ChannelId,
    pub channel_mod_polls: ChannelId,

    pub attachment_cache_path: PathBuf,
    pub attachment_cache_max_size: usize,

    pub time_started: chrono::DateTime<chrono::Utc>,
}

impl Config {
    fn from_environment() -> Result<Self> {
        Ok(Config {
            discord_token: required_env_var("TOKEN")?,
            guild: GuildId(parse_required_env_var("GUILD")?),
            role_mod: RoleId(parse_required_env_var("ROLE_MOD")?),
            role_helper: RoleId(parse_required_env_var("ROLE_HELPER")?),
            role_mute: RoleId(parse_required_env_var("ROLE_MUTE")?),
            roles_color: required_env_var("ROLES_COLOR")?
                .split(',')
                .map(|x| Ok(RoleId(x.trim().parse()?)))
                .collect::<Result<_>>()?,
            category_mod_private: ChannelId(parse_required_env_var("CATEGORY_MOD_PRIVATE")?),
            channel_showcase: ChannelId(parse_required_env_var("CHANNEL_SHOWCASE")?),
            channel_feedback: ChannelId(parse_required_env_var("CHANNEL_FEEDBACK")?),
            channel_modlog: ChannelId(parse_required_env_var("CHANNEL_MODLOG")?),
            channel_auto_mod: ChannelId(parse_required_env_var("CHANNEL_AUTO_MOD")?),
            channel_mod_bot_stuff: ChannelId(parse_required_env_var("CHANNEL_MOD_BOT_STUFF")?),
            channel_bot_messages: ChannelId(parse_required_env_var("CHANNEL_BOT_MESSAGES")?),
            channel_bot_traffic: ChannelId(parse_required_env_var("CHANNEL_BOT_TRAFFIC")?),
            channel_tech_support: ChannelId(parse_required_env_var("CHANNEL_TECH_SUPPORT")?),
            channel_mod_polls: ChannelId(parse_required_env_var("CHANNEL_MOD_POLLS")?),
            attachment_cache_path: parse_required_env_var("ATTACHMENT_CACHE_PATH")?,
            attachment_cache_max_size: parse_required_env_var("ATTACHMENT_CACHE_MAX_SIZE")?,
            time_started: chrono::Utc::now(),
        })
    }

    async fn log_bot_action<F>(&self, ctx: &client::Context, build_embed: F)
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let result = self
            .guild
            .send_embed(&ctx, self.channel_modlog, build_embed)
            .await;

        log_error!(result);
    }
    async fn log_automod_action<F>(&self, ctx: &client::Context, build_embed: F)
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let result = self
            .guild
            .send_embed(&ctx, self.channel_auto_mod, build_embed)
            .await;
        log_error!(result);
    }

    #[allow(unused)]
    async fn is_mod(&self, ctx: &client::Context, user_id: UserId) -> Result<bool> {
        let user = user_id.to_user(&ctx).await?;
        Ok(user.has_role(&ctx, self.guild, self.role_mod).await?)
    }
}

impl TypeMapKey for Config {
    type Value = Arc<Config>;
}

pub struct FrameworkKey;
impl TypeMapKey for FrameworkKey {
    type Value = Arc<StandardFramework>;
}

#[tokio::main]
async fn main() {
    let honeycomb_api_key = std::env::var("HONEYCOMB_API_KEY").ok();

    init_tracing(honeycomb_api_key.clone());
    if let Some(honeycomb_api_key) = honeycomb_api_key {
        send_honeycomb_deploy_marker(&honeycomb_api_key).await;
    }

    let span = tracing::span!(Level::DEBUG, "main");
    let _enter = span.enter();

    init_cpu_logging().await;

    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();

    let config = Config::from_environment().expect("Failed to load experiment");

    let db = Db::new().await.expect("Failed to initialize database");
    db.run_migrations().await.unwrap();
    db.remove_forbidden_highlights().await.unwrap();

    // we're manually calling the framework, to only run commands if none of our
    // message_create event handler filters say no.
    // currently, serenity still _requires_ a framework to be configured as long as the feature is enabled,
    // thus this stub framework is necessary for now.
    // Soon, with a rework of the command framework, this will be solved.
    let stub_framework = StandardFramework::new();

    let mut client = Client::builder(&config.discord_token)
        .event_handler(events::Handler)
        .framework(stub_framework)
        .intents(GatewayIntents::all())
        .await
        .expect("Error creating client");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!").delimiters(vec![" ", "\n"]))
        .on_dispatch_error(dispatch_error_hook)
        .before(before)
        .after(after)
        .group(&MODERATOR_GROUP)
        .group(&HELPERORMOD_GROUP)
        .group(&GENERAL_GROUP)
        .help(&help::MY_HELP);

    client.cache_and_http.cache.set_max_messages(500);

    {
        let mut data = client.data.write().await;
        data.insert::<Config>(Arc::new(config));
        data.insert::<Db>(Arc::new(db));
        data.insert::<FrameworkKey>(Arc::new(framework));
    };

    if let Err(why) = client.start().await {
        tracing::error!("An error occurred while running the client: {:?}", why);
    }
}

fn init_tracing(honeycomb_api_key: Option<String>) {
    let log_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::try_new("robbb=trace,serenity=debug,serenity::http::ratelimiting=off")
                .unwrap()
        })
        .add_directive("robbb=trace".parse().unwrap());

    let sub = tracing_subscriber::registry()
        .with(log_filter)
        .with(tracing_subscriber::fmt::Layer::default());

    if let Some(api_key) = honeycomb_api_key {
        tracing::error!("honeycomb api key is set, initializing honeycomb layer");
        let config = libhoney::Config {
            options: libhoney::client::Options {
                api_key,
                dataset: "robbb".to_string(),
                ..libhoney::client::Options::default()
            },
            transmission_options: libhoney::transmission::Options::default(),
        };
        let sub = sub.with(tracing_honeycomb::Builder::new_libhoney("robbb", config).build());
        tracing::subscriber::set_global_default(sub).expect("setting default subscriber failed");
    } else {
        tracing::info!("no honeycomb api key is set");
        let sub = sub.with(tracing_honeycomb::new_blackhole_telemetry_layer());
        tracing::subscriber::set_global_default(sub).expect("setting default subscriber failed");
    };
}

#[hook]
async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
    tracing::debug!(
        command_name,
        msg.content = %msg.content,
        msg.author = %msg.author,
        msg.id = %msg.id,
        msg.channel_id = %msg.channel_id,
        "command '{}' invoked by '{}'",
        command_name,
        msg.author.tag()
    );
    true
}

#[hook]
#[tracing::instrument(skip_all, fields(%msg.content, %msg.channel_id, error.command_name = %_command_name, %error))]
async fn dispatch_error_hook(
    ctx: &client::Context,
    msg: &Message,
    error: DispatchError,
    _command_name: &str,
) {
    // Log dispatch errors that should be logged
    match &error {
        DispatchError::CheckFailed(required, Reason::Log(log))
        | DispatchError::CheckFailed(required, Reason::UserAndLog { user: _, log }) => {
            tracing::warn!("Check for {} failed with: {}", required, log);
        }
        _ => {}
    };

    let _ = msg.reply_error(&ctx, display_dispatch_error(error)).await;
}

fn display_dispatch_error(err: DispatchError) -> String {
    match err {
        DispatchError::CheckFailed(_required, reason) => match reason {
            Reason::User(reason)
            | Reason::UserAndLog {
                user: reason,
                log: _,
            } => reason,
            _ => "You're not allowed to use this command".to_string(),
        },
        DispatchError::Ratelimited(_info) => "Hit a rate-limit".to_string(),
        DispatchError::CommandDisabled => "Command is disabled".to_string(),
        DispatchError::BlockedUser => "User not allowed to use bot".to_string(),
        DispatchError::BlockedGuild => "Guild is blocked by bot".to_string(),
        DispatchError::BlockedChannel => "Channel is blocked by bot".to_string(),
        DispatchError::OnlyForDM => "Command may only be used in DMs".to_string(),
        DispatchError::OnlyForGuilds => "Command may only be used in a server".to_string(),
        DispatchError::OnlyForOwners => "Command may only be used by owners".to_string(),
        DispatchError::LackingRole => "Missing a required role".to_string(),
        DispatchError::LackingPermissions(flags) => format!(
            "User is missing permissions - required permission number is {}",
            flags
        ),
        DispatchError::NotEnoughArguments { min, given } => format!(
            "Not enough arguments provided - got {} but needs {}",
            given, min
        ),
        DispatchError::TooManyArguments { max, given } => format!(
            "Too many arguments provided - got {} but can only handle {}",
            given, max
        ),
        _ => {
            tracing::error!("Unhandled dispatch error: {:?}", err);
            "Failed to run command".to_string()
        }
    }
}

#[hook]
async fn after(ctx: &client::Context, msg: &Message, command_name: &str, result: CommandResult) {
    match result {
        Err(err) => match err.downcast_ref::<UserErr>() {
            Some(err) => match err {
                UserErr::MentionedUserNotFound => {
                    let _ = msg.reply_error(&ctx, "No user found with that name").await;
                }
                UserErr::InvalidUsage(usage) => {
                    let _ = msg.reply_error(&ctx, format!("Usage: {}", usage)).await;
                }
                UserErr::Other(issue) => {
                    let _ = msg.reply_error(&ctx, format!("Error: {}", issue)).await;
                }
            },
            None => match err.downcast::<serenity::Error>() {
                Ok(err) => {
                    let err = *err;
                    tracing::warn!(
                        error.command_name = %command_name,
                        error.message = %err,
                        "Serenity error [handling {}]: {} ({:?})",
                        command_name,
                        &err,
                        &err
                    );
                    match err {
                        serenity::Error::Http(err) => {
                            if let serenity::http::error::Error::UnsuccessfulRequest(res) = *err {
                                if res.status_code == serenity::http::StatusCode::NOT_FOUND
                                    && res.error.message.to_lowercase().contains("unknown user")
                                {
                                    let _ = msg.reply_error(&ctx, "User not found").await;
                                } else {
                                    let _ = msg.reply_error(&ctx, "Something went wrong").await;
                                }
                            }
                        }
                        serenity::Error::Model(err) => {
                            let _ = msg.reply_error(&ctx, err).await;
                        }
                        _ => {
                            let _ = msg.reply_error(&ctx, "Something went wrong").await;
                        }
                    }
                }
                Err(err) => {
                    let _ = msg.reply_error(&ctx, "Something went wrong").await;
                    tracing::warn!(
                        error.command_name = %command_name,
                        error.message = %err,
                        "Internal error [handling {}]: {} ({:#?})",
                        command_name,
                        &err,
                        &err
                    );
                }
            },
        },
        Ok(()) => {}
    }
}

async fn send_honeycomb_deploy_marker(api_key: &str) {
    let client = reqwest::Client::new();
    log_error!(
        client
            .post("https://api.honeycomb.io/1/markers/robbb")
            .header("X-Honeycomb-Team", api_key)
            .body(format!(
                r#"{{"message": "{}", "type": "deploy"}}"#,
                util::bot_version()
            ))
            .send()
            .await
    );
}

async fn init_cpu_logging() {
    use cpu_monitor::CpuInstant;
    use std::time::Duration;
    tokio::spawn(
        async {
            loop {
                let start = CpuInstant::now();
                tokio::time::sleep(Duration::from_millis(4000)).await;
                let end = CpuInstant::now();
                if let (Ok(start), Ok(end)) = (start, end) {
                    let duration = end - start;
                    let percentage = duration.non_idle() * 100.;
                    tracing::info!(cpu_usage = percentage);
                }
            }
        }
        .instrument(tracing::info_span!("cpu-usage")),
    );
}
