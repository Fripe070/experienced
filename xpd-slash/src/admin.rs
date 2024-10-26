use std::{borrow::Cow, fmt::Display};

use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_common::CURRENT_GIT_SHA;

use crate::{
    cmd_defs::{
        admin::{
            self, AdminCommandBanGuild, AdminCommandGuildStats, AdminCommandLeave,
            AdminCommandPardonGuild, AdminCommandResetGuild, AdminCommandResetUser,
            AdminCommandSetNick,
        },
        AdminCommand,
    },
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_admin(
    data: AdminCommand,
    guild_id: Id<GuildMarker>,
    invoker: Id<UserMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    if guild_id != state.control_guild {
        return Err(Error::NotControlGuild);
    };
    if !state.owners.contains(&invoker) {
        return Err(Error::NotControlUser);
    }
    let contents = match data {
        AdminCommand::Leave(lg) => leave_guild(state, lg).await,
        AdminCommand::ResetGuild(rg) => reset_guild(state, rg).await,
        AdminCommand::ResetUser(ru) => reset_user(state, ru).await,
        AdminCommand::SetNick(sn) => set_nick(state, sn).await,
        AdminCommand::BanGuild(bg) => ban_guild(state, bg).await,
        AdminCommand::PardonGuild(pg) => pardon_guild(state, pg).await,
        AdminCommand::GuildStats(gs) => get_guild_stats(state, gs).await,
        AdminCommand::Stats(admin::AdminCommandStats) => get_bot_stats(state).await,
    }?;
    Ok(XpdSlashResponse::new()
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn leave_guild(state: SlashState, leave: AdminCommandLeave) -> Result<String, Error> {
    let guild: Id<GuildMarker> = leave.guild.parse()?;
    state.client.leave_guild(guild).await?;
    Ok(format!("Left guild {guild}"))
}

async fn reset_guild(state: SlashState, leave: AdminCommandResetGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = leave.guild.parse()?;
    let rows = xpd_database::delete_levels_guild(&state.db, guild).await?;
    Ok(format!(
        "Reset levels for guild {guild}. It had {rows} users worth of data."
    ))
}

async fn reset_user(state: SlashState, leave: AdminCommandResetUser) -> Result<String, Error> {
    let rows = xpd_database::delete_levels_user(&state.db, leave.user).await?;
    Ok(format!(
        "Reset your levels. They had level data in {rows} guilds."
    ))
}

async fn set_nick(state: SlashState, nick: AdminCommandSetNick) -> Result<String, Error> {
    let guild: Id<GuildMarker> = nick.guild.parse()?;
    state
        .client
        .update_current_member(guild)
        .nick(nick.name.as_deref())
        .await?;
    Ok(format!(
        "Set nickname to {} in {guild}",
        nick.name.unwrap_or_else(|| "{default}".to_string())
    ))
}

async fn ban_guild(state: SlashState, ban: AdminCommandBanGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = ban.guild.parse()?;
    xpd_database::ban_guild(&state.db, guild, ban.duration).await?;
    Ok(format!("Banned guild {guild}"))
}

async fn pardon_guild(state: SlashState, pardon: AdminCommandPardonGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = pardon.guild.parse()?;
    xpd_database::pardon_guild(&state.db, guild).await?;
    Ok(format!("Pardoned guild {guild}"))
}

async fn get_guild_stats(state: SlashState, gs: AdminCommandGuildStats) -> Result<String, Error> {
    let guild_id: Id<GuildMarker> = gs.guild.parse()?;
    let levels = xpd_database::levels_in_guild(&state.db, guild_id).await?;

    let guild = state
        .client
        .guild(guild_id)
        .with_counts(true)
        .await?
        .model()
        .await?;

    let large = if guild.large { "large" } else { "" };
    let name = &guild.name;
    let online = fmt_opt_u64(guild.approximate_presence_count);
    let members = fmt_opt_u64(guild.approximate_member_count);

    Ok(format!(
        "{levels} levels in database for {large} guild {name}. Roughly {online} members online of {members} total members.",
    ))
}

fn fmt_opt_u64(item: Option<u64>) -> impl Display {
    item.map_or_else(|| Cow::Borrowed("unknown"), |v| Cow::Owned(v.to_string()))
}

async fn get_bot_stats(state: SlashState) -> Result<String, Error> {
    let levels_held = xpd_database::total_levels(&state.db).await?;
    Ok(format!(
        "Roughly {levels_held} levels in database. Bot version `git-{CURRENT_GIT_SHA}`"
    ))
}
