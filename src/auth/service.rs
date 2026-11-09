use super::jwt::*;
use super::*;
pub async fn login(pool: Arc<Pool>, payload: LoginPayload) -> Result<AuthBodyDto, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError("Database unavailable".into()))?;
    let user = crate::services::users::find_user_by_email_or_username(&mut conn, &payload.user)
        .await?
        .ok_or(ModuleError::AuthError)?;
    if !crate::helpers::password_verfier(&payload.password, &user.password) {
        return Err(ModuleError::AuthError);
    }
    create_session_token(user.id, user.role, user.organisation_id)
}
