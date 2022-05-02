use poise::serenity_prelude::ApplicationCommand;

use super::*;

#[poise::command(slash_command, prefix_command, hide_in_help)]
pub async fn register(
    ctx: Ctx<'_>,
    //#[description = "global"]
    //#[flag]
    //global: bool,
) -> Res<()> {
    if let Some(guild) = ctx.guild() {
        let commands = guild.get_application_commands(ctx.discord()).await?;
        for command in commands {
            guild
                .delete_application_command(ctx.discord(), command.id)
                .await?;
        }

        //let new_commands = &ctx.framework().options().commands;
        //let commands_builder = poise::builtins::create_application_commands(new_commands);
        //println!("{:?}", new_commands);

        //guild
        //.set_application_commands(ctx.discord(), |b| {
        //*b = commands_builder;
        //b
        //})
        //.await?;
    }
    let global_commands =
        ApplicationCommand::get_global_application_commands(ctx.discord()).await?;
    for command in global_commands {
        ApplicationCommand::delete_global_application_command(ctx.discord(), command.id).await?;
    }

    poise::builtins::register_application_commands(ctx, false).await?;

    Ok(())
}
