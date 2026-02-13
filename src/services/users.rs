use super::*;
use crate::{dto::users::*,};

pub async fn register_user(pool: Arc<Pool>, payload: NewUser) -> Result<Message, ModuleError> {
    let user = payload.build()?;
    let mut conn = &mut pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    diesel::insert_into(schema::users::table)
        .values(&user)
        .execute(&mut conn)
        .await?;

    Ok("User registered successfully".into())
}

pub async fn find_user_by_email_or_username(
    conn: &mut crate::Connection<'_>,
    identifier: &str,
) -> Result<Option<UserDto>, ModuleError> {
    let user = schema::users::table
        .filter(
            schema::users::email
                .eq(identifier)
                .or(schema::users::username.eq(identifier)),
        )
        .select(UserDto::as_select())
        .first::<UserDto>(conn)
        .await
        .optional()?;

    Ok(user)
}

// pub async fn find_user_by_email_or_username(
//     conn: &mut crate::Connection<'_>,
//     identifier: &str,
// ) -> Result<Option<User>, ModuleError> {

//     let user = schema::users::table
//         .filter(
//             schema::users::email
//                 .eq(identifier)
//                 .or(schema::users::username.eq(identifier)),
//         )
//         .first::<User>(conn)
//         .await
//         .optional()
//         .map_err(|_| ModuleError::InternalError("could not get connection from db"))?;

//     Ok(user)
// }
