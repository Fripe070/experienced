use redis::AsyncCommands;
use twilight_model::{guild::Member, user::User};

use crate::Error;

pub async fn set_chunk(redis: deadpool_redis::Pool, chunk: Vec<Member>) -> Result<(), Error> {
    let mut user_pairs: Vec<(String, String)> = Vec::with_capacity(chunk.len());
    for member in chunk {
        user_pairs.push((
            format!("cache-user-{}", member.user.id.get()),
            serde_json::to_string(&member.user)?,
        ));
    }
    Ok(redis
        .get()
        .await?
        .set_multiple::<String, String, ()>(user_pairs.as_slice())
        .await?)
}

pub async fn set_user(redis: deadpool_redis::Pool, user: &User) -> Result<(), Error> {
    Ok(redis
        .get()
        .await?
        .set::<String, String, ()>(
            format!("cache-user-{}", user.id.get()),
            serde_json::to_string(user)?,
        )
        .await?)
}