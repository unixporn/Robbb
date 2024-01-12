use crate::{extensions::PoiseContextExt, log_error, prelude::Ctx, util::ellipsis_text};

use anyhow::Result;
use itertools::Itertools;
use poise::{
    serenity_prelude::{CreateActionRow, UserId},
    CreateReply,
};
use serenity::{
    builder::{
        CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
        EditMessage,
    },
    client,
    model::channel::Message,
};

const PAGINATION_LEFT: &str = "LEFT";
const PAGINATION_RIGHT: &str = "RIGHT";
const MAX_EMBED_FIELDS: usize = 12; // discords max is 25, but that's ugly

#[derive(Debug)]
pub struct PaginatedEmbed {
    pages: Vec<CreateEmbed>,
    base_embed: CreateEmbed,
}

impl PaginatedEmbed {
    pub async fn create(
        embeds: impl IntoIterator<Item = CreateEmbed>,
        base_embed: CreateEmbed,
    ) -> PaginatedEmbed {
        PaginatedEmbed { pages: embeds.into_iter().collect(), base_embed }
    }

    pub async fn create_from_fields(
        title: String,
        fields: impl IntoIterator<Item = (String, String)>,
        base_embed: CreateEmbed,
    ) -> PaginatedEmbed {
        let pages = fields.into_iter().chunks(MAX_EMBED_FIELDS);
        let pages: Vec<_> = pages.into_iter().collect();
        let page_cnt = pages.len();
        let pages = pages
            .into_iter()
            .enumerate()
            .map(|(page_idx, fields)| {
                let mut e = base_embed.clone();
                if page_cnt < 2 {
                    e = e.title(&title);
                } else {
                    e = e.title(format!("{} ({}/{})", title, page_idx + 1, page_cnt));
                }
                e.fields(fields.map(|(k, v)| (k, ellipsis_text(&v, 500), false)).collect_vec())
            })
            .collect_vec();

        PaginatedEmbed { pages, base_embed }
    }

    //#[tracing::instrument(name = "send_paginated_embed", skip_all, fields(paginated_embed.page_cnt = %self.pages.len()))]
    pub async fn reply_to(&self, ctx: Ctx<'_>, ephemeral: bool) -> Result<Message> {
        let pages = self.pages.clone();
        match pages.len() {
            0 => {
                let handle = ctx.reply_embed_full(ephemeral, self.base_embed.clone()).await?;
                Ok(handle.message().await?.into_owned())
            }
            1 => {
                let page = self.pages.first().unwrap();
                let handle = ctx.reply_embed_full(ephemeral, page.clone()).await?;
                Ok(handle.message().await?.into_owned())
            }
            _ => {
                let created_msg_handle = ctx
                    .send(
                        CreateReply::default()
                            .ephemeral(ephemeral)
                            .components(vec![make_paginate_row(0, pages.len())])
                            .embed(self.pages.first().unwrap().clone()),
                    )
                    .await?;
                let created_msg = created_msg_handle.message().await?.into_owned();

                tokio::spawn({
                    let serenity_ctx = ctx.serenity_context().clone();
                    let user_id = ctx.author().id;
                    let created_msg = created_msg.clone();
                    async move {
                        log_error!(
                            handle_pagination_interactions(
                                &serenity_ctx,
                                pages,
                                user_id,
                                created_msg
                            )
                            .await
                        )
                    }
                });

                Ok(created_msg)
            }
        }
    }
}

#[tracing::instrument(skip_all)]
async fn handle_pagination_interactions(
    serenity_ctx: &client::Context,
    pages: Vec<CreateEmbed>,
    user_id: UserId,
    mut created_msg: Message,
) -> Result<()> {
    let mut current_page_idx = 0;

    let mut interactions = crate::collect_interaction::await_component_interactions_by(
        serenity_ctx,
        &created_msg,
        user_id,
        10,
        std::time::Duration::from_secs(30),
    );

    while let Some(interaction) = interactions.next().await {
        let direction = interaction.data.clone().custom_id;
        if direction == PAGINATION_LEFT && current_page_idx > 0 {
            current_page_idx -= 1;
        } else if direction == PAGINATION_RIGHT && current_page_idx < pages.len() - 1 {
            current_page_idx += 1;
        }
        interaction
            .create_response(
                &serenity_ctx,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default()
                        .embed(pages.get(current_page_idx).unwrap().clone())
                        .components(vec![make_paginate_row(current_page_idx, pages.len())]),
                ),
            )
            .await?;
    }
    created_msg
        .edit(
            &serenity_ctx,
            EditMessage::default().embed(pages.get(current_page_idx).unwrap().clone()),
        )
        .await?;
    Ok(())
}

fn make_paginate_row(page_idx: usize, page_cnt: usize) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(PAGINATION_LEFT).label("←").disabled(page_idx == 0),
        CreateButton::new(PAGINATION_RIGHT).label("→").disabled(page_idx >= page_cnt - 1),
    ])
}
