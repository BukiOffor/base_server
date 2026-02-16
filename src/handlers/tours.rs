use super::*;
use crate::auth::jwt::Claims;
use crate::dto::tours::*;

pub fn routes(state: Arc<AppState>) -> Router {
    let routes = tour_routes(state.clone());
    let api = Router::new().nest("/tours", routes);
    Router::new().merge(api)
}

pub fn tour_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_tour).get(list_all_tours))
        .route("/org/{org_id}", get(list_tours_by_org))
        .route("/graph/{org_id}", get(get_tour_graph))
        .route("/{id}", delete(delete_tour))
        .route("/links", post(create_tour_link))
        .route("/links/{id}", delete(delete_tour_link))
        .with_state(state)
}

pub async fn create_tour(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewTour>,
) -> Result<Json<TourDto>, ModuleError> {
    let tour = services::tours::create_tour(state.pool.clone(), payload, claims.user_id).await?;
    Ok(Json(tour))
}

pub async fn list_all_tours(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TourDto>>, ModuleError> {
    let tours = services::tours::list_all_tours(state.pool.clone()).await?;
    Ok(Json(tours))
}

pub async fn list_tours_by_org(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<uuid::Uuid>,
) -> Result<Json<Vec<TourDto>>, ModuleError> {
    let tours = services::tours::list_tours_by_org(state.pool.clone(), org_id).await?;
    Ok(Json(tours))
}

pub async fn get_tour_graph(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<uuid::Uuid>,
) -> Result<Json<TourGraph>, ModuleError> {
    let graph = services::tours::get_tour_graph(state.pool.clone(), org_id).await?;
    Ok(Json(graph))
}

pub async fn delete_tour(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Message>, ModuleError> {
    let response = services::tours::delete_tour(state.pool.clone(), id).await?;
    Ok(Json(response))
}

pub async fn create_tour_link(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewTourLink>,
) -> Result<Json<TourLinkDto>, ModuleError> {
    let link = services::tours::create_tour_link(state.pool.clone(), payload).await?;
    Ok(Json(link))
}

pub async fn delete_tour_link(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Message>, ModuleError> {
    let response = services::tours::delete_tour_link(state.pool.clone(), id).await?;
    Ok(Json(response))
}
