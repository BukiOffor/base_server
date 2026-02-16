use super::*;
use crate::dto::tours::*;

pub async fn create_tour(
    pool: Arc<Pool>,
    payload: NewTour,
    user_id: uuid::Uuid,
) -> Result<TourDto, ModuleError> {
    let tour = payload.build(user_id);
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let result = diesel::insert_into(schema::tours::table)
        .values(&tour)
        .returning(TourDto::as_returning())
        .get_result::<TourDto>(&mut conn)
        .await?;

    Ok(result)
}

pub async fn list_tours_by_org(
    pool: Arc<Pool>,
    org_id: uuid::Uuid,
) -> Result<Vec<TourDto>, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let tours = schema::tours::table
        .filter(schema::tours::organisation_id.eq(org_id))
        .select(TourDto::as_select())
        .order(schema::tours::created_at.desc())
        .load::<TourDto>(&mut conn)
        .await?;

    Ok(tours)
}

pub async fn list_all_tours(pool: Arc<Pool>) -> Result<Vec<TourDto>, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let tours = schema::tours::table
        .select(TourDto::as_select())
        .order(schema::tours::created_at.desc())
        .load::<TourDto>(&mut conn)
        .await?;

    Ok(tours)
}

pub async fn get_tour_graph(pool: Arc<Pool>, org_id: uuid::Uuid) -> Result<TourGraph, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    // Get all tours (nodes) for this organisation
    let tours = schema::tours::table
        .filter(schema::tours::organisation_id.eq(org_id))
        .select(TourDto::as_select())
        .load::<TourDto>(&mut conn)
        .await?;

    let tour_ids: Vec<uuid::Uuid> = tours.iter().map(|t| t.id).collect();

    // Get all links where source is one of these tours
    let links = schema::tour_links::table
        .filter(schema::tour_links::source_node_id.eq_any(&tour_ids))
        .select(TourLinkDto::as_select())
        .load::<TourLinkDto>(&mut conn)
        .await?;

    // Build the graph
    let rooms: Vec<TourGraphNode> = tours
        .into_iter()
        .map(|tour| {
            let tour_links: Vec<TourGraphLink> = links
                .iter()
                .filter(|l| l.source_node_id == tour.id)
                .map(|l| TourGraphLink {
                    target: l.target_node_id,
                    yaw: l.yaw,
                    pitch: l.pitch,
                    label: l.label.clone(),
                })
                .collect();

            TourGraphNode {
                id: tour.id,
                name: tour.name,
                panorama_url: tour.panorama_url,
                links: tour_links,
            }
        })
        .collect();

    Ok(TourGraph {
        organisation_id: org_id,
        rooms,
    })
}

pub async fn delete_tour(pool: Arc<Pool>, tour_id: uuid::Uuid) -> Result<Message, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let deleted = diesel::delete(schema::tours::table.filter(schema::tours::id.eq(tour_id)))
        .execute(&mut conn)
        .await?;

    if deleted == 0 {
        return Err(ModuleError::ItemNotFound);
    }

    Ok("Tour deleted successfully".into())
}

pub async fn create_tour_link(
    pool: Arc<Pool>,
    payload: NewTourLink,
) -> Result<TourLinkDto, ModuleError> {
    let link = payload.build();
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    // Verify both source and target exist
    let source_exists = schema::tours::table
        .filter(schema::tours::id.eq(link.source_node_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await?;

    if source_exists == 0 {
        return Err(ModuleError::Error("Source tour node does not exist".into()));
    }

    let target_exists = schema::tours::table
        .filter(schema::tours::id.eq(link.target_node_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await?;

    if target_exists == 0 {
        return Err(ModuleError::Error("Target tour node does not exist".into()));
    }

    let result = diesel::insert_into(schema::tour_links::table)
        .values(&link)
        .returning(TourLinkDto::as_returning())
        .get_result::<TourLinkDto>(&mut conn)
        .await?;

    Ok(result)
}

pub async fn delete_tour_link(
    pool: Arc<Pool>,
    link_id: uuid::Uuid,
) -> Result<Message, ModuleError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ModuleError::InternalError(POOL_ERROR_MSG.into()))?;

    let deleted =
        diesel::delete(schema::tour_links::table.filter(schema::tour_links::id.eq(link_id)))
            .execute(&mut conn)
            .await?;

    if deleted == 0 {
        return Err(ModuleError::ItemNotFound);
    }

    Ok("Link deleted successfully".into())
}
