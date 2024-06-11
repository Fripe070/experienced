#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
    str::FromStr,
};

use simpleinterpolation::Interpolation;
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::Intents,
    guild::Member,
    id::{
        marker::{ChannelMarker, RoleMarker, UserMarker},
        Id,
    },
    user::User,
    util::ImageHash,
};

pub trait DisplayName {
    #[must_use]
    fn display_name(&self) -> &str;
}

impl DisplayName for User {
    fn display_name(&self) -> &str {
        self.global_name.as_ref().unwrap_or(&self.name)
    }
}

impl DisplayName for Member {
    fn display_name(&self) -> &str {
        self.nick
            .as_deref()
            .unwrap_or_else(|| self.user.display_name())
    }
}

impl DisplayName for MemberDisplayInfo {
    fn display_name(&self) -> &str {
        self.nick.as_ref().map_or_else(
            || {
                self.global_name
                    .as_ref()
                    .map_or(self.name.as_str(), |global| global.as_str())
            },
            |nick| nick.as_str(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberDisplayInfo {
    pub id: Id<UserMarker>,
    pub name: String,
    pub global_name: Option<String>,
    pub nick: Option<String>,
    pub avatar: Option<ImageHash>,
    pub local_avatar: Option<ImageHash>,
    pub bot: bool,
}

impl From<User> for MemberDisplayInfo {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            name: value.name,
            global_name: value.global_name,
            nick: None,
            avatar: value.avatar,
            local_avatar: None,
            bot: value.bot,
        }
    }
}

impl From<Member> for MemberDisplayInfo {
    fn from(value: Member) -> Self {
        Self {
            id: value.user.id,
            name: value.user.name,
            global_name: value.user.global_name,
            nick: value.nick,
            avatar: value.user.avatar,
            local_avatar: value.avatar,
            bot: value.user.bot,
        }
    }
}

impl MemberDisplayInfo {
    #[must_use]
    pub fn with_nick(self, nick: Option<String>) -> Self {
        Self { nick, ..self }
    }
}

/// Get environment variable and parse it, panicking on failure
/// # Panics
/// If the environment variable cannot be found or parsed
#[must_use]
pub fn parse_var<T>(key: &str) -> T
where
    T: FromStr,
    T::Err: Display,
{
    get_var(key)
        .parse()
        .unwrap_or_else(|e| panic!("{key} could not be parsed: {e}"))
}

/// Get environment variable and parse it, panicking on failure
/// # Panics
/// If the environment variable cannot be found or parsed
#[must_use]
pub fn get_var(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|e| panic!("Expected {key} in environment: {e}"))
}

pub trait ReinterpretPrimitiveBits<O> {
    fn reinterpret_bits(&self) -> O;
}

macro_rules! impl_primitive_reinterpret {
    ($from:ty, $to:ty) => {
        impl ReinterpretPrimitiveBits<$to> for $from {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            fn reinterpret_bits(&self) -> $to {
                *self as $to
            }
        }
    };
}

impl_primitive_reinterpret!(u8, i8);
impl_primitive_reinterpret!(u16, i16);
impl_primitive_reinterpret!(u32, i32);
impl_primitive_reinterpret!(u64, i64);
impl_primitive_reinterpret!(u128, i128);
impl_primitive_reinterpret!(i8, u8);
impl_primitive_reinterpret!(i16, u16);
impl_primitive_reinterpret!(i32, u32);
impl_primitive_reinterpret!(i64, u64);
impl_primitive_reinterpret!(i128, u128);

/// Fetches the raw ID data from Twilight and returns it as an i64, so it can be stored in Postgres
/// easily.
/// Essentially a no-op.
#[must_use]
#[inline]
pub fn id_to_db<T>(id: Id<T>) -> i64 {
    id.get().reinterpret_bits()
}

/// Create a new checked twilight id from an i64. Only get this from the DB!
/// Essentially a no-op.
#[inline]
#[must_use]
pub fn db_to_id<T>(db: i64) -> Id<T> {
    Id::new(db.reinterpret_bits())
}

pub const TEMPLATE_VARIABLES: [&str; 2] = ["user_mention", "level"];

#[derive(Clone, Default)]
pub struct RawGuildConfig {
    pub one_at_a_time: Option<bool>,
    pub level_up_message: Option<String>,
    pub level_up_channel: Option<i64>,
}

impl TryFrom<RawGuildConfig> for GuildConfig {
    type Error = simpleinterpolation::Error;

    fn try_from(value: RawGuildConfig) -> Result<Self, Self::Error> {
        let level_up_message = if let Some(str) = value.level_up_message {
            Some(Interpolation::new(str)?)
        } else {
            None
        };

        let gc = Self {
            one_at_a_time: value.one_at_a_time,
            level_up_message,
            level_up_channel: value.level_up_channel.map(db_to_id),
        };
        Ok(gc)
    }
}

#[derive(Default, Debug)]
pub struct GuildConfig {
    pub one_at_a_time: Option<bool>,
    pub level_up_message: Option<Interpolation>,
    pub level_up_channel: Option<Id<ChannelMarker>>,
}

#[derive(Debug)]
pub struct RoleReward {
    pub id: Id<RoleMarker>,
    pub requirement: i64,
}

#[must_use]
#[inline]
pub fn sort_rewards(a: &RoleReward, b: &RoleReward) -> std::cmp::Ordering {
    a.requirement.cmp(&b.requirement)
}

#[inline]
const fn tribool(data: Option<bool>) -> &'static str {
    match data {
        None => "unset",
        Some(true) => "true",
        Some(false) => "false",
    }
}

fn opt_code_str(data: Option<&str>) -> Cow<str> {
    data.map_or(Cow::Borrowed("unset"), |v| Cow::Owned(format!("`{v}`")))
}

fn opt_mention_str<T>(data: Option<Id<T>>, mention_kind: char) -> Cow<'static, str> {
    data.map_or(Cow::Borrowed("unset"), |v| {
        Cow::Owned(format!("`<{mention_kind}{v}>`"))
    })
}

impl Display for GuildConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "One reward role at a time: {}",
            tribool(self.one_at_a_time)
        )?;
        write!(
            f,
            "Level-up message: {}",
            opt_code_str(
                self.level_up_message
                    .as_ref()
                    .map(Interpolation::input_value)
                    .as_deref()
            )
        )?;
        write!(
            f,
            "Level-up channel: {}",
            opt_mention_str(self.level_up_channel, '#')
        )?;
        Ok(())
    }
}

pub trait RequiredEvents {
    fn required_intents() -> Intents;
    fn required_events() -> EventTypeFlags;
}
