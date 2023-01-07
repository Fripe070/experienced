use crate::AppState;
use sqlx::query;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{
            application_command::{CommandData, CommandOptionValue},
            Interaction, InteractionData, InteractionType,
        },
    },
    channel::message::MessageFlags,
    http::{
        attachment::Attachment,
        interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    },
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub async fn process(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    Ok(if interaction.kind == InteractionType::ApplicationCommand {
        InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(process_app_cmd(interaction, state).await?),
        }
    } else {
        InteractionResponse {
            kind: InteractionResponseType::Pong,
            data: None,
        }
    })
}

async fn process_app_cmd(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    #[cfg(debug_assertions)]
    println!("DEBUG: {interaction:#?}");
    let data = if let Some(data) = interaction.data {
        if let InteractionData::ApplicationCommand(cmd) = data {
            *cmd
        } else {
            return Err(CommandProcessorError::WrongInteractionData);
        }
    } else {
        return Err(CommandProcessorError::NoInteractionData);
    };
    let invoker = match interaction.member {
        Some(val) => val.user,
        None => interaction.user,
    }
    .ok_or(CommandProcessorError::NoInvoker)?;
    match data.kind {
        CommandType::ChatInput => {
            process_slash_cmd(data, interaction.guild_id, invoker, state).await
        }
        CommandType::User => process_user_cmd(data, invoker, state).await,
        CommandType::Message => process_msg_cmd(data, invoker, state).await,
        _ => Err(CommandProcessorError::WrongInteractionData),
    }
}

async fn process_slash_cmd(
    data: CommandData,
    guild_id: Option<Id<GuildMarker>>,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    match data.name.as_str() {
        "rank" | "level" => {
            for option in &data.options {
                if option.name == "user" {
                    if let CommandOptionValue::User(user_id) = option.value {
                        let user = data
                            .resolved
                            .as_ref()
                            .ok_or(CommandProcessorError::NoResolvedData)?
                            .users
                            .get(&user_id)
                            .ok_or(CommandProcessorError::NoTarget)?;
                        return get_level(user, &invoker, state).await;
                    };
                }
            }
            get_level(&invoker, &invoker, state).await
        }
        "xp" => Ok(crate::manager::process_xp(data, guild_id, &invoker, state).await?),
        _ => Err(CommandProcessorError::UnrecognizedCommand),
    }
}

async fn process_user_cmd(
    data: CommandData,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    let msg_id = data
        .target_id
        .ok_or(CommandProcessorError::NoMessageTargetId)?;
    let user = data
        .resolved
        .as_ref()
        .ok_or(CommandProcessorError::NoResolvedData)?
        .users
        .get(&msg_id.cast())
        .ok_or(CommandProcessorError::NoTarget)?;
    get_level(user, &invoker, state).await
}

async fn process_msg_cmd(
    data: CommandData,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    let msg_id = data
        .target_id
        .ok_or(CommandProcessorError::NoMessageTargetId)?;
    let user = &data
        .resolved
        .as_ref()
        .ok_or(CommandProcessorError::NoResolvedData)?
        .messages
        .get(&msg_id.cast())
        .ok_or(CommandProcessorError::NoTarget)?
        .author;
    get_level(user, &invoker, state).await
}

async fn get_level(
    user: &User,
    invoker: &User,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    // Select current XP from the database, return 0 if there is no row
    let xp = match query!("SELECT xp FROM levels WHERE id = ?", user.id.get())
        .fetch_one(&state.db)
        .await
    {
        Ok(val) => val.xp,
        Err(e) => match e {
            sqlx::Error::RowNotFound => 0,
            _ => Err(e)?,
        },
    };
    let rank = query!("SELECT COUNT(*) as count FROM levels WHERE xp > ?", xp)
        .fetch_one(&state.db)
        .await?
        .count
        + 1;
    let level_info = mee6::LevelInfo::new(xp);
    let content = if user.bot {
        "Bots aren't ranked, that would be silly!".to_string()
    } else if invoker == user {
        if xp == 0 {
            "You aren't ranked yet, because you haven't sent any messages!".to_string()
        } else {
            return generate_level_response(user, level_info, rank).await;
        }
    } else if xp == 0 {
        format!(
            "{}#{} isn't ranked yet, because they haven't sent any messages!",
            user.name,
            user.discriminator()
        )
    } else {
        return generate_level_response(user, level_info, rank).await;
    };
    Ok(InteractionResponseDataBuilder::new()
        .flags(MessageFlags::EPHEMERAL)
        .content(content)
        .build())
}

async fn generate_level_response(
    user: &User,
    level_info: mee6::LevelInfo,
    rank: i64,
) -> Result<InteractionResponseData, CommandProcessorError> {
    Ok(InteractionResponseDataBuilder::new()
        .attachments(vec![Attachment {
            description: Some("Rank card".to_string()),
            file: crate::render_card::render(
                user.name.clone(),
                user.discriminator().to_string(),
                level_info.level().to_string(),
                rank.to_string(),
                level_info.percentage(),
            )
            .await?,
            filename: "card.png".to_string(),
            id: 0,
        }])
        .build())
}

#[derive(Debug, thiserror::Error)]
pub enum CommandProcessorError {
    #[error("Discord sent a command that is not known!")]
    UnrecognizedCommand,
    #[error("Discord did not send a user object for the command invoker when it was required!")]
    NoInvoker,
    #[error("Discord did not send a user object for the command target when it was required!")]
    NoTarget,
    #[error("Discord did not send part of the Resolved Data!")]
    NoResolvedData,
    #[error("Discord did not send target ID for message!")]
    NoMessageTargetId,
    #[error("Discord sent interaction data for an unsupported interaction type!")]
    WrongInteractionData,
    #[error("Discord did not send any interaction data!")]
    NoInteractionData,
    #[error("XP subprocessor encountered an error: {0}!")]
    XpSubprocessor(#[from] crate::manager::Error),
    #[error("SVG renderer encountered an error: {0}!")]
    ImageGenerator(#[from] crate::render_card::RenderingError),
    #[error("SQLx encountered an error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
