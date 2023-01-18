use twilight_interactions::command::{CommandOption, CreateOption};

const DEFAULT_IMPORTANT: Color = Color::new(255, 255, 255);
const DEFAULT_SECONDARY: Color = Color::new(204, 204, 204);
const DEFAULT_RANK: Color = Color::new(255, 255, 255);
const DEFAULT_LEVEL: Color = Color::new(143, 202, 92);
const DEFAULT_BORDER: Color = Color::new(133, 79, 43);
const DEFAULT_BACKGROUND: Color = Color::new(97, 55, 31);
const DEFAULT_PROGRESS_FOREGROUND: Color = Color::new(71, 122, 30);
const DEFAULT_PROGRESS_BACKGROUND: Color = Color::new(143, 202, 92);

#[derive(serde::Serialize, Debug, Clone, Copy)]
pub struct Colors {
    pub important: Color,
    pub secondary: Color,
    pub rank: Color,
    pub level: Color,
    pub border: Color,
    pub background: Color,
    pub progress_foreground: Color,
    pub progress_background: Color,
}

impl CommandOption for Color {
    fn from_option(
        value: twilight_model::application::interaction::application_command::CommandOptionValue,
        _data: twilight_interactions::command::internal::CommandOptionData,
        _resolved: Option<&twilight_model::application::interaction::application_command::CommandInteractionDataResolved>,
    ) -> Result<Self, twilight_interactions::error::ParseOptionErrorType> {
        if let twilight_model::application::interaction::application_command::CommandOptionValue::String(string) = value {
            Ok(Self::from_hex(&string).map_err(|e| twilight_interactions::error::ParseOptionErrorType::InvalidChoice(format!("{e}")))?)
        } else {
            Err(twilight_interactions::error::ParseOptionErrorType::InvalidType(value.kind()))
        }
    }
}

impl CreateOption for Color {
    fn create_option(
        data: twilight_interactions::command::internal::CreateOptionData,
    ) -> twilight_model::application::command::CommandOption {
        twilight_model::application::command::CommandOption {
            autocomplete: Some(data.autocomplete),
            channel_types: None,
            choices: None,
            description: data.description,
            description_localizations: data.description_localizations,
            kind: twilight_model::application::command::CommandOptionType::String,
            max_length: Some(7),
            max_value: None,
            min_length: Some(6),
            min_value: None,
            name: data.name,
            name_localizations: data.name_localizations,
            options: None,
            required: data.required,
        }
    }
}

impl Colors {
    pub async fn for_user(
        db: &sqlx::PgPool,
        id: twilight_model::id::Id<twilight_model::id::marker::UserMarker>,
    ) -> Self {
        #[allow(clippy::cast_possible_wrap)]
        let colors = if let Ok(colors) =
            sqlx::query!("SELECT * FROM custom_card WHERE id = $1", id.get() as i64)
                .fetch_one(db)
                .await
        {
            colors
        } else {
            return Self::default();
        };
        Self {
            important: crate::from_maybe_hex!(colors.important, DEFAULT_IMPORTANT),
            secondary: crate::from_maybe_hex!(colors.secondary, DEFAULT_SECONDARY),
            rank: crate::from_maybe_hex!(colors.rank, DEFAULT_RANK),
            level: crate::from_maybe_hex!(colors.level, DEFAULT_LEVEL),
            border: crate::from_maybe_hex!(colors.border, DEFAULT_BORDER),
            background: crate::from_maybe_hex!(colors.background, DEFAULT_BACKGROUND),
            progress_foreground: crate::from_maybe_hex!(
                colors.progress_foreground,
                DEFAULT_PROGRESS_FOREGROUND
            ),
            progress_background: crate::from_maybe_hex!(
                colors.progress_background,
                DEFAULT_PROGRESS_BACKGROUND
            ),
        }
    }
}
impl std::fmt::Display for Colors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::add_output!(f, "Important text", self.important, DEFAULT_IMPORTANT);
        crate::add_output!(f, "Secondary text", self.secondary, DEFAULT_SECONDARY);
        crate::add_output!(f, "Rank", self.rank, DEFAULT_RANK);
        crate::add_output!(f, "Level", self.level, DEFAULT_LEVEL);
        crate::add_output!(f, "Border", self.border, DEFAULT_BORDER);
        crate::add_output!(f, "Background", self.background, DEFAULT_BACKGROUND);
        crate::add_output!(
            f,
            "Progress bar completed",
            self.progress_foreground,
            DEFAULT_PROGRESS_FOREGROUND
        );
        crate::add_output!(
            f,
            "Progress bar remaining",
            self.progress_background,
            DEFAULT_PROGRESS_BACKGROUND
        );
        Ok(())
    }
}

#[macro_export]
macro_rules! add_output {
    ($f:expr, $name:expr, $val:expr, $default:expr) => {
        write!($f, "{}: `{}`", $name, $val)?;
        if $val == $default {
            writeln!($f, " (default)")?;
        } else {
            writeln!($f)?;
        };
    };
}

#[macro_export]
macro_rules! from_maybe_hex {
    ($val:expr, $default:expr) => {
        $val.map_or($default, |color| {
            Color::from_hex(&color).unwrap_or($default)
        })
    };
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            important: DEFAULT_IMPORTANT,
            secondary: DEFAULT_SECONDARY,
            rank: DEFAULT_RANK,
            level: DEFAULT_LEVEL,
            border: DEFAULT_BORDER,
            background: DEFAULT_BACKGROUND,
            progress_foreground: DEFAULT_PROGRESS_FOREGROUND,
            progress_background: DEFAULT_PROGRESS_BACKGROUND,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    /// Takes hex-color input and converts it to a Color.
    pub fn from_hex(hex: &impl ToString) -> Result<Self, Error> {
        let hex = hex.to_string();
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(Error::InvalidLength);
        }
        Ok(Self {
            red: u8::from_str_radix(&hex[0..=1], 16)?,
            green: u8::from_str_radix(&hex[2..=3], 16)?,
            blue: u8::from_str_radix(&hex[4..=5], 16)?,
        })
    }
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid length! Hex data length must be exactly 6 characters!")]
    InvalidLength,
    #[error("Integer parsing error: {0}!")]
    ParseInt(#[from] std::num::ParseIntError),
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }
}

impl serde::Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
