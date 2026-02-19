use super::*;
use crate::dto::projects::*;

pub async fn create_project(
    pool: Arc<Pool>,
    payload: NewProject,
) -> Result<ProjectDto, ModuleError> {
    let project = payload.build();
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let result = diesel::insert_into(schema::projects::table)
        .values(&project)
        .returning(ProjectDto::as_returning())
        .get_result::<ProjectDto>(&mut conn)
        .await?;

    Ok(result)
}

pub async fn list_projects_by_org(
    pool: Arc<Pool>,
    org_id: uuid::Uuid,
) -> Result<Vec<ProjectDto>, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let projects = schema::projects::table
        .filter(schema::projects::organisation_id.eq(org_id))
        .select(ProjectDto::as_select())
        .order(schema::projects::created_at.desc())
        .load::<ProjectDto>(&mut conn)
        .await?;

    Ok(projects)
}

pub async fn get_project_by_id(
    pool: Arc<Pool>,
    project_id: uuid::Uuid,
) -> Result<ProjectDto, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let project = schema::projects::table
        .filter(schema::projects::id.eq(project_id))
        .select(ProjectDto::as_select())
        .first::<ProjectDto>(&mut conn)
        .await
        .optional()?
        .ok_or(ModuleError::ItemNotFound)?;

    Ok(project)
}

pub async fn update_project_visibility(
    pool: Arc<Pool>,
    project_id: uuid::Uuid,
    is_public: bool,
) -> Result<ProjectDto, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let result =
        diesel::update(schema::projects::table.filter(schema::projects::id.eq(project_id)))
            .set(schema::projects::is_public.eq(is_public))
            .returning(ProjectDto::as_returning())
            .get_result::<ProjectDto>(&mut conn)
            .await?;

    Ok(result)
}

pub async fn delete_project(
    pool: Arc<Pool>,
    project_id: uuid::Uuid,
) -> Result<Message, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let deleted =
        diesel::delete(schema::projects::table.filter(schema::projects::id.eq(project_id)))
            .execute(&mut conn)
            .await?;

    if deleted == 0 {
        return Err(ModuleError::ItemNotFound);
    }

    Ok("Project deleted successfully".into())
}
