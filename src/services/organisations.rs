use super::*;
use crate::dto::organisations::*;

pub async fn create_organisation(
    pool: Arc<Pool>,
    payload: NewOrganisation,
) -> Result<Message, ModuleError> {
    let org = payload.build();
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    diesel::insert_into(schema::organisations::table)
        .values(&org)
        .execute(&mut conn)
        .await?;

    Ok("Organisation created successfully".into())
}

pub async fn list_organisations(pool: Arc<Pool>) -> Result<Vec<OrganisationDto>, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let orgs = schema::organisations::table
        .select(OrganisationDto::as_select())
        .order(schema::organisations::created_at.desc())
        .load::<OrganisationDto>(&mut conn)
        .await?;

    Ok(orgs)
}

pub async fn get_organisation_by_id(
    pool: Arc<Pool>,
    org_id: uuid::Uuid,
) -> Result<OrganisationDto, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let org = schema::organisations::table
        .filter(schema::organisations::id.eq(org_id))
        .select(OrganisationDto::as_select())
        .first::<OrganisationDto>(&mut conn)
        .await
        .optional()?
        .ok_or(ModuleError::ItemNotFound)?;

    Ok(org)
}
