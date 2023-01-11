use std::collections::HashMap;

use robbb_db::{fetch::Fetch, fetch_field::FetchField};

use super::*;

/// Fetch a users system information.
#[poise::command(slash_command, guild_only, prefix_command, rename = "fetch")]
pub async fn fetch(
    ctx: Ctx<'_>,
    #[description = "The user"] user: Option<Member>,
    #[description = "The specific field you care about"] field: Option<FetchField>,
) -> Res<()> {
    let db = ctx.get_db();
    let user = member_or_self(ctx, user).await?;
    ctx.defer().await?;

    // Query the database
    let fetch_info: Fetch = db.get_fetch(user.user.id).await?.unwrap_or_else(|| Fetch {
        user: user.user.id,
        info: HashMap::new(),
        create_date: None,
    });

    let create_date = fetch_info.create_date;
    let fetch_data: Vec<(FetchField, String)> = fetch_info.get_values_ordered();
    let color = user.colour(ctx.serenity_context());

    match field {
        // Handle fetching a single field
        Some(desired_field) => {
            let (field_name, value) = fetch_data
                .into_iter()
                .find(|(k, _)| k == &desired_field)
                .user_error("Failed to get that value. Maybe the user hasn't set it?")?;
            ctx.send_embed(|e| {
                e.author(|a| a.name(user.user.tag()).icon_url(user.user.face()));
                e.title(format!("{}'s {}", user.user.name, field_name));
                e.color_opt(color);
                if let Some(date) = create_date {
                    e.timestamp(date);
                }
                if desired_field == FetchField::Image {
                    e.image(value);
                } else if let Some(value) = format_fetch_field_value(&field_name, value) {
                    e.description(value);
                } else {
                    e.description("Not set");
                }
            })
            .await?;
        }

        // Handle fetching all fields
        None => {
            ctx.send_embed(|e| {
                e.author_user(&user.user);
                e.color_opt(color);
                if let Some(date) = create_date {
                    e.timestamp(date);
                }

                for (key, value) in fetch_data {
                    if key == FetchField::Image {
                        e.image(value);
                    } else {
                        if key == FetchField::Distro {
                            if let Some(url) = find_distro_image(&value) {
                                e.thumbnail(url);
                            }
                        }
                        if let Some(val) = format_fetch_field_value(&key, value) {
                            e.field(key, val, true);
                        }
                    }
                }
            })
            .await?;
        }
    }

    Ok(())
}
