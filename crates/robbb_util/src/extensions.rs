use crate::{config::Config, embeds, log_error, prelude::Ctx, UpEmotes};

use anyhow::{Context, Result};
use itertools::Itertools;
use poise::{CreateReply, ReplyHandle};
use robbb_db::Db;
use serenity::{
    async_trait,
    builder::{
        CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage,
        CreateThread,
    },
    client::{self, Cache},
    model::{
        channel::{GuildChannel, Message},
        guild::Emoji,
        id::{ChannelId, EmojiId, GuildId},
        prelude::User,
        Colour, Timestamp,
    },
    prelude::Mentionable,
};
use std::{collections::HashMap, fmt::Display, sync::Arc};

type StdResult<T, E> = std::result::Result<T, E>;

#[extend::ext(name = PoiseContextExt)]
#[async_trait]
pub impl<'a> Ctx<'a> {
    fn get_config(&self) -> Arc<Config> {
        self.data().config.clone()
    }

    fn get_db(&self) -> Arc<Db> {
        self.data().db.clone()
    }

    fn get_up_emotes(&self) -> Option<Arc<UpEmotes>> {
        self.data().up_emotes.read().clone()
    }

    fn is_prefix(&self) -> bool {
        matches!(self, poise::Context::Prefix(_))
    }

    /// Reply with an ephemeral embed.
    async fn reply_embed_ephemeral_builder(
        &self,
        build: impl FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        self.reply_embed_ephemeral(build(embeds::base_embed(self))).await
    }

    /// Reply with an embed.
    async fn reply_embed_builder(
        &self,
        build: impl FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        self.reply_embed(build(embeds::base_embed(self))).await
    }

    /// Reply with an embed.
    async fn reply_embed(&self, embed: CreateEmbed) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        let reply = CreateReply::default().ephemeral(false).embed(embed).reply(true);
        self.send(reply).await
    }

    /// Reply with an ephemeral embed.
    async fn reply_embed_ephemeral(
        &self,
        embed: CreateEmbed,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        let reply = CreateReply::default().ephemeral(true).embed(embed).reply(true);
        self.send(reply).await
    }

    async fn say_success(
        &self,
        text: impl Display + Send + Sync + 'static,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        tracing::info!(
            msg.ephemeral = true,
            msg.content = %text,
            msg.responding_to_user = %self.author().tag(),
            "Sending success message to user"
        );
        self.reply_embed_ephemeral(
            embeds::make_success_embed(self.serenity_context(), &text.to_string()).await,
        )
        .await
    }

    async fn say_error(
        &self,
        text: impl Display + Send + Sync + 'static,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        tracing::info!(
            msg.ephemeral = true,
            msg.content = %text,
            msg.responding_to_user = %self.author().tag(),
            "Sending error message to user"
        );
        self.reply_embed_ephemeral(
            embeds::make_error_embed(self.serenity_context(), &text.to_string()).await,
        )
        .await
    }
    async fn say_success_mod_action(
        &self,
        text: impl Display + Send + Sync + 'static,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        tracing::info!(
            msg.ephemeral = false,
            msg.content = %text,
            msg.responding_to_user = %self.author().tag(),
            "Sending success_mod_action message to user"
        );
        self.reply_embed(
            embeds::make_success_mod_action_embed(self.serenity_context(), &text.to_string()).await,
        )
        .await
    }

    async fn guild_channel(&self) -> anyhow::Result<GuildChannel> {
        Ok(self
            .channel_id()
            .to_channel(&self.serenity_context())
            .await
            .context("Failed to load channel")?
            .guild()
            .context("Failed to load GuildChannel")?)
    }

    fn get_guild_emojis(&self) -> Option<HashMap<EmojiId, Emoji>> {
        Some(self.guild()?.emojis.clone())
    }

    fn get_random_stare(&self) -> Option<Emoji> {
        self.data().up_emotes.read().clone().and_then(|x| x.random_stare())
    }
}

#[extend::ext(name = ClientContextExt)]
#[async_trait]
pub impl client::Context {
    async fn get_guild_emojis(&self, id: GuildId) -> Result<HashMap<EmojiId, Emoji>> {
        if let Some(up_emotes) = self.get_up_emotes().await {
            Ok(up_emotes.all_emoji.clone())
        } else {
            tracing::info!("Requesting guild emotes from discord");
            Ok(self.http.get_guild(id).await?.emojis)
        }
    }

    async fn get_up_emotes(&self) -> Option<Arc<UpEmotes>> {
        self.data.read().await.get::<UpEmotes>().cloned()
    }

    async fn get_config_and_db(&self) -> (Arc<Config>, Arc<Db>) {
        tokio::join!(self.get_config(), self.get_db())
    }

    async fn get_config(&self) -> Arc<Config> {
        self.data.read().await.get::<Config>().cloned().unwrap()
    }
    async fn get_db(&self) -> Arc<Db> {
        self.data.read().await.get::<Db>().cloned().unwrap()
    }

    async fn log_bot_action(
        &self,
        build_embed: impl FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    ) {
        let config = self.get_config().await;
        log_error!(config.guild.send_embed(self, config.channel_modlog, build_embed).await);
    }

    async fn log_automod_action(
        &self,
        build_embed: impl FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    ) {
        let config = self.get_config().await;
        log_error!(config.guild.send_embed(self, config.channel_auto_mod, build_embed).await);
    }
}

#[extend::ext]
pub impl User {
    /// Format a user as `name#discriminator (user-id)`
    fn name_with_disc_and_id(&self) -> String {
        format!("{} ({})", self.tag(), self.id)
    }
    /// Format a user as `@mention (name#discriminator)`
    /// Primarily needed because discord on mobile is bad and doesn't show mentions of users if they're not cached.
    fn mention_and_tag(&self) -> String {
        format!("{} ({})", self.mention(), self.tag())
    }
}

#[extend::ext]
#[async_trait]
pub impl GuildId {
    async fn send_embed(
        &self,
        ctx: &client::Context,
        channel_id: ChannelId,
        build: impl FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    ) -> Result<Message> {
        let embed = build(embeds::base_embed_ctx(ctx).await);
        Ok(channel_id
            .send_message(&ctx, CreateMessage::default().embed(embed))
            .await
            .context("Failed to send embed message")?)
    }
}

#[extend::ext]
#[async_trait]
pub impl Message {
    fn find_image_urls(&self) -> Vec<String> {
        self.embeds
            .iter()
            .filter_map(|embed| embed.image.clone())
            .map(|image| image.url)
            .chain(
                self.attachments
                    .iter()
                    .filter(|a| a.dimensions().is_some() && crate::util::is_image_file(&a.filename))
                    .map(|a| a.url.to_string()),
            )
            .collect_vec()
    }

    async fn reply_embed(
        &self,
        ctx: &client::Context,
        build: impl FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    ) -> Result<Message> {
        self.channel_id
            .send_message(
                &ctx,
                CreateMessage::default()
                    .allowed_mentions(CreateAllowedMentions::default().replied_user(false))
                    .reference_message(self)
                    .embed(build(embeds::base_embed_ctx(ctx).await)),
            )
            .await
            .context("Failed to send embed")
    }

    async fn reply_error(
        &self,
        ctx: &client::Context,
        text: impl Display + Send + Sync + 'static,
    ) -> Result<Message> {
        let embed = embeds::make_error_embed(ctx, &text.to_string()).await;
        self.reply_embed(ctx, |_| embed).await
    }

    async fn create_thread(
        &self,
        ctx: &client::Context,
        title: impl Display + Send + Sync + 'static,
    ) -> Result<GuildChannel> {
        self.channel(&ctx)
            .await
            .context("Failed to fetch message channel")?
            .guild()
            .context("Failed to request guild channel")?
            .create_thread_from_message(&ctx, self, CreateThread::new(title.to_string()))
            .await
            .context("Failed to create a thread")
    }

    fn to_context_link(&self) -> String {
        format!("[(context)]({})", self.link())
    }
}

#[extend::ext]
#[async_trait]
pub impl ChannelId {
    async fn send_embed(&self, ctx: &client::Context, embed: CreateEmbed) -> Result<Message> {
        let msg = CreateMessage::default().embed(embed);
        Ok(self.send_message(&ctx, msg).await.context("Failed to send embed message")?)
    }

    async fn send_embed_builder(
        &self,
        ctx: &client::Context,
        build: impl FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    ) -> Result<Message> {
        let msg = CreateMessage::default().embed(build(embeds::base_embed_ctx(ctx).await));
        Ok(self.send_message(&ctx, msg).await.context("Failed to send embed message")?)
    }

    fn name_cached(&self, cache: &Cache) -> Option<String> {
        self.to_channel_cached(cache).map(|c| c.name().to_string())
    }

    fn name_cached_or_fallback(&self, cache: &Cache) -> String {
        self.name_cached(cache).unwrap_or_else(|| "non-cached channel".to_string())
    }
}

#[extend::ext]
pub impl CreateEmbed {
    fn field_opt(
        self,
        name: impl Into<String>,
        value: Option<impl Into<String>>,
        inline: bool,
    ) -> Self {
        let Some(value) = value else { return self };
        self.field(name, value, inline)
    }

    fn color_opt(self, x: Option<impl Into<Colour>>) -> CreateEmbed {
        let Some(x) = x else { return self };
        self.color(x)
    }

    fn timestamp_opt(self, x: Option<impl Into<Timestamp>>) -> CreateEmbed {
        let Some(x) = x else { return self };
        self.timestamp(x)
    }

    fn author_icon(self, name: impl Into<String>, icon_url: impl Into<String>) -> Self {
        self.author(CreateEmbedAuthor::new(name).icon_url(icon_url))
    }

    fn footer_str(self, name: impl Into<String>) -> Self {
        self.footer(CreateEmbedFooter::new(name))
    }

    fn author_user(self, u: &User) -> Self {
        self.author(
            CreateEmbedAuthor::new(u.tag())
                .icon_url(u.face())
                .url(format!("https://discord.com/users/{}", u.id)),
        )
    }

    /// Create a [`CreateMessage`] that only contains this embed.
    fn into_create_message(self) -> CreateMessage {
        CreateMessage::default().embed(self)
    }
}

#[extend::ext(name = StrExt)]
pub impl<T: AsRef<str>> T {
    fn split_once_at(&self, c: char) -> Option<(&str, &str)> {
        let s: &str = self.as_ref();
        let index = s.find(c)?;
        Some((&s[..index], &s[index + c.len_utf8()..]))
    }

    /// Splits the string into two parts, separated by the given word.
    /// Ex. `"foo bar baz".split_at_word("bar") // ---> ("foo", "baz")`
    fn split_at_word(&self, split_at: &str) -> (String, String) {
        let mut words = self.as_ref().trim().split(' ').collect_vec();
        match words.iter().position(|w| w == &split_at) {
            Some(word_ind) => {
                let right_side = words.split_off(word_ind + 1).join(" ");
                words.pop();
                (words.join(" "), right_side)
            }
            None => (String::from(self.as_ref()), String::new()),
        }
    }
}
