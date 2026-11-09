use super::*;
use axum::{
    Extension,
    routing::{delete, get, patch, post, put},
};
pub fn routes(state: Arc<AppState>) -> Router {
    let router = Router::new()
        .route(
            "/public/current-event",
            get(handle).layer(Extension("public_current")),
        )
        .route(
            "/public/lookup",
            post(handle).layer(Extension("public_lookup")),
        )
        .route(
            "/public/events/{eventId}/map",
            get(handle).layer(Extension("public_map")),
        )
        .route(
            "/public/assignments/{id}/confirm",
            post(handle).layer(Extension("public_confirm")),
        )
        .route(
            "/public/assignments/{id}/unavailable",
            post(handle).layer(Extension("public_unavailable")),
        )
        .route(
            "/public/assignments/{id}/available",
            post(handle).layer(Extension("public_available")),
        )
        .route(
            "/public/settings",
            get(handle).layer(Extension("public_settings")),
        )
        .route(
            "/team/auth/request-code",
            post(handle).layer(Extension("team_auth_code")),
        )
        .route(
            "/team/auth/verify",
            post(handle).layer(Extension("team_auth_verify")),
        )
        .route(
            "/team/auth/logout",
            post(handle).layer(Extension("team_auth_logout")),
        )
        .route("/team/me", get(handle).layer(Extension("team_me")))
        .route(
            "/team/events/{eventId}/map",
            get(handle).layer(Extension("team_map")),
        )
        .route("/auth/login", post(handle).layer(Extension("auth_login")))
        .route(
            "/auth/refresh",
            post(handle).layer(Extension("auth_refresh")),
        )
        .route("/auth/logout", post(handle).layer(Extension("auth_logout")))
        .route("/auth/me", get(handle).layer(Extension("auth_me")))
        .route(
            "/auth/forgot-password",
            post(handle).layer(Extension("auth_forgot")),
        )
        .route(
            "/auth/reset-password",
            post(handle).layer(Extension("auth_reset")),
        )
        .route("/events", get(handle).layer(Extension("event_list")))
        .route("/events", post(handle).layer(Extension("event_create")))
        .route(
            "/events/{eventId}",
            get(handle).layer(Extension("event_get")),
        )
        .route(
            "/events/{eventId}",
            patch(handle).layer(Extension("event_patch")),
        )
        .route(
            "/events/{eventId}",
            delete(handle).layer(Extension("event_delete")),
        )
        .route(
            "/events/{eventId}/duplicate",
            post(handle).layer(Extension("event_duplicate")),
        )
        .route(
            "/events/{eventId}/archive",
            post(handle).layer(Extension("event_archive")),
        )
        .route(
            "/events/{eventId}/publish",
            post(handle).layer(Extension("event_publish")),
        )
        .route(
            "/events/{eventId}/unpublish",
            post(handle).layer(Extension("event_unpublish")),
        )
        .route(
            "/settings/current-event",
            put(handle).layer(Extension("settings_current")),
        )
        .route(
            "/events/{eventId}/zones",
            get(handle).layer(Extension("zone_list")),
        )
        .route(
            "/events/{eventId}/zones",
            post(handle).layer(Extension("zone_create")),
        )
        .route(
            "/events/{eventId}/zones/order",
            put(handle).layer(Extension("zone_order")),
        )
        .route(
            "/events/{eventId}/zones/apply-template",
            post(handle).layer(Extension("zone_template")),
        )
        .route("/zones/{id}", patch(handle).layer(Extension("zone_patch")))
        .route(
            "/zones/{id}",
            delete(handle).layer(Extension("zone_delete")),
        )
        .route(
            "/zones/{id}/positions",
            post(handle).layer(Extension("position_create")),
        )
        .route(
            "/zones/{id}/positions/order",
            put(handle).layer(Extension("position_order")),
        )
        .route(
            "/positions/{id}",
            patch(handle).layer(Extension("position_patch")),
        )
        .route(
            "/positions/{id}",
            delete(handle).layer(Extension("position_delete")),
        )
        .route(
            "/venue/template",
            get(handle).layer(Extension("venue_template")),
        )
        .route("/members", get(handle).layer(Extension("member_list")))
        .route("/members", post(handle).layer(Extension("member_create")))
        .route(
            "/members/check-phone",
            get(handle).layer(Extension("member_check")),
        )
        .route("/members/{id}", get(handle).layer(Extension("member_get")))
        .route(
            "/members/{id}",
            patch(handle).layer(Extension("member_patch")),
        )
        .route(
            "/members/{id}",
            delete(handle).layer(Extension("member_delete")),
        )
        .route(
            "/members/bulk-delete",
            post(handle).layer(Extension("member_bulk_delete")),
        )
        .route(
            "/members/{id}/role",
            put(handle).layer(Extension("member_role")),
        )
        .route("/team", get(handle).layer(Extension("team_tree")))
        .route("/zones/names", get(handle).layer(Extension("zones_names")))
        .route(
            "/events/{eventId}/assignments",
            get(handle).layer(Extension("assignment_list")),
        )
        .route(
            "/events/{eventId}/assignments",
            post(handle).layer(Extension("assignment_create")),
        )
        .route(
            "/events/{eventId}/assignments/bulk",
            post(handle).layer(Extension("assignment_bulk")),
        )
        .route(
            "/events/{eventId}/assignments/bulk-remove",
            post(handle).layer(Extension("assignment_bulk_remove")),
        )
        .route(
            "/events/{eventId}/assignments/copy",
            post(handle).layer(Extension("assignment_copy")),
        )
        .route(
            "/events/{eventId}/roster/copy-from",
            post(handle).layer(Extension("roster_copy")),
        )
        .route(
            "/events/{eventId}/absences",
            get(handle).layer(Extension("absences")),
        )
        .route(
            "/assignments/{id}",
            patch(handle).layer(Extension("assignment_patch")),
        )
        .route(
            "/assignments/{id}",
            delete(handle).layer(Extension("assignment_delete")),
        )
        .route(
            "/assignments/{id}/confirm",
            post(handle).layer(Extension("assignment_confirm")),
        )
        .route("/events/{eventId}/map", get(handle).layer(Extension("map")))
        .route(
            "/events/{eventId}/overview",
            get(handle).layer(Extension("overview")),
        )
        .route("/search", get(handle).layer(Extension("search")))
        .route("/activity", get(handle).layer(Extension("activity")))
        .route(
            "/notifications",
            get(handle).layer(Extension("notifications")),
        )
        .route(
            "/notifications/read-all",
            post(handle).layer(Extension("notifications_read_all")),
        )
        .route(
            "/notifications/{id}/read",
            post(handle).layer(Extension("notification_read")),
        )
        .route(
            "/imports/template",
            get(handle).layer(Extension("imports_template")),
        )
        .route("/imports/preview", post(import_preview))
        .route(
            "/imports/{id}/commit",
            post(handle).layer(Extension("imports_commit")),
        )
        .route(
            "/events/{eventId}/exports/{kind}",
            get(handle).layer(Extension("export")),
        )
        .route(
            "/events/{eventId}/reports/summary",
            get(handle).layer(Extension("report_summary")),
        )
        .route("/settings", get(handle).layer(Extension("settings_get")))
        .route(
            "/settings",
            patch(handle).layer(Extension("settings_patch")),
        );
    let router = if std::env::var("APP_ENV").as_deref() == Ok("development") {
        router.route(
            "/admin/reset-demo",
            post(handle).layer(Extension("admin_reset")),
        )
    } else {
        router
    };
    router
        .fallback(|| async { ApiError::missing() })
        .with_state(state)
}
async fn handle(
    State(state): State<Arc<AppState>>,
    Extension(op): Extension<&'static str>,
    Path(params): Path<HashMap<String, String>>,
    query: std::result::Result<Query<ListQuery>, axum::extract::rejection::QueryRejection>,
    headers: HeaderMap,
    request: Request,
) -> Result<Response> {
    let Query(query) = query.map_err(|_| ApiError::bad("Invalid query parameters"))?;
    entry(state, op, params, query, headers, request).await
}
async fn import_preview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: axum::extract::Multipart,
) -> Result<Response> {
    let mut c = state
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(503, "DATABASE_UNAVAILABLE", "Database unavailable"))?;
    let identity = admin::authenticate(&mut c, &headers, "admin").await?;
    let mut event = None;
    let mut file = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad("Invalid multipart upload"))?
    {
        match field.name() {
            Some("eventId") => {
                event = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| ApiError::bad("Invalid eventId"))?,
                )
            }
            Some("file") => {
                let name = field.file_name().unwrap_or("upload").to_owned();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| ApiError::bad("File exceeds upload limit"))?;
                file = Some((name, bytes));
            }
            _ => {}
        }
    }
    let event = event.ok_or_else(|| ApiError::bad("eventId is required"))?;
    let (name, bytes) = file.ok_or_else(|| ApiError::bad("file is required"))?;
    let _ = identity;
    c.transaction::<Response, ApiError, _>(|c| {
        Box::pin(async move { Ok(ok(svc::imports::preview(c, &name, &bytes, &event).await?)) })
            .into()
    })
    .await
}
