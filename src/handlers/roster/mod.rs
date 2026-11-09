//! Explicit API routes; transport concerns stay here and business rules in services.
mod assignments;
mod events;
mod members;
mod operations;
mod public;
mod routes;
use crate::{
    AppState,
    auth::admin,
    dto::roster::ListQuery,
    errors::api::{ApiError, Result},
    models::roster::{self as db, Entity},
    services::roster::{self as svc, required, s},
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Query, Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use diesel_async::{AsyncConnection, RunQueryDsl};
pub use routes::routes;
use serde_json::{Value, json};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
fn ok(v: Value) -> Response {
    Json(v).into_response()
}
fn created(v: Value) -> Response {
    (StatusCode::CREATED, Json(v)).into_response()
}
fn empty() -> Response {
    StatusCode::NO_CONTENT.into_response()
}
fn param<'a>(p: &'a HashMap<String, String>, k: &str) -> Result<&'a str> {
    p.get(k).map(String::as_str).ok_or_else(ApiError::missing)
}
fn download(d: svc::exports::Download) -> Result<Response> {
    let disposition =
        axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", d.filename))
            .map_err(|_| ApiError::bad("Invalid filename"))?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(d.content_type),
            ),
            (axum::http::header::CONTENT_DISPOSITION, disposition),
        ],
        d.bytes,
    )
        .into_response())
}
async fn entry(
    state: Arc<AppState>,
    op: &'static str,
    params: HashMap<String, String>,
    mut query: ListQuery,
    headers: HeaderMap,
    request: Request,
) -> Result<Response> {
    if let Ok(url) = reqwest::Url::parse(&format!("http://localhost{}", request.uri())) {
        query.zone_names = url
            .query_pairs()
            .filter(|(k, _)| k == "zoneName")
            .map(|(_, v)| v.into_owned())
            .collect();
    }
    if query.page.is_some_and(|page| page > i64::MAX as usize) {
        return Err(ApiError::bad("page is too large"));
    }
    if query
        .dir
        .as_deref()
        .is_some_and(|d| !matches!(d, "asc" | "desc"))
    {
        return Err(ApiError::bad("dir must be asc or desc"));
    }
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|a| a.0.ip().to_string())
        .unwrap_or("unknown".into());
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(503, "DATABASE_UNAVAILABLE", "Database unavailable"))?;
    let public = op.starts_with("public_");
    let team = matches!(op, "team_me" | "team_map");
    let anon = public
        || matches!(
            op,
            "auth_login"
                | "auth_refresh"
                | "auth_forgot"
                | "auth_reset"
                | "team_auth_code"
                | "team_auth_verify"
        );
    if public || op.starts_with("auth_") || op.starts_with("team_auth_") {
        admin::rate(
            &mut conn,
            &format!("ip:{op}:{ip}"),
            if public { 20 } else { 30 },
            60,
        )
        .await?;
    }
    let mut actor = String::new();
    let mut role = String::new();
    let mut identity = None;
    if !anon {
        let i = admin::authenticate(
            &mut conn,
            &headers,
            if team || op == "team_auth_logout" {
                "team"
            } else {
                "admin"
            },
        )
        .await?;
        if i.kind == "admin" {
            let user = admin::admin(&mut conn, &i).await?;
            actor = s(&user, "name").to_owned();
            role = s(&user, "role").to_owned();
        } else {
            let m = db::get(&mut conn, Entity::Member, &i.sub).await?;
            if !matches!(s(&m, "role"), "coordinator" | "leader") {
                return Err(ApiError::forbidden());
            }
        }
        identity = Some(i);
    }
    let write = !matches!(request.method(), &axum::http::Method::GET);
    if role == "editor"
        && write
        && (matches!(
            op,
            "event_publish"
                | "event_unpublish"
                | "event_delete"
                | "event_archive"
                | "settings_patch"
                | "settings_current"
                | "admin_reset"
        ))
    {
        return Err(ApiError::forbidden());
    }
    let body = if op == "imports_preview" {
        Value::Null
    } else {
        let bytes = axum::body::to_bytes(request.into_body(), 5 * 1024 * 1024 + 16384)
            .await
            .map_err(|_| ApiError::bad("Request body is too large"))?;
        if bytes.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&bytes)?
        }
    };
    crate::dto::roster::validate(op, &body)?;
    // Per-account throttles are committed even if the authentication attempt fails.
    match op {
        "auth_login" | "auth_forgot" => {
            admin::rate(
                &mut conn,
                &format!("{op}:{}", s(&body, "email").trim().to_lowercase()),
                if op == "auth_login" { 10 } else { 3 },
                900,
            )
            .await?;
        }
        "team_auth_code" | "team_auth_verify" => {
            let phone = svc::normalize_phone(required(&body, "phone")?)?;
            admin::rate(
                &mut conn,
                &format!("{op}:{phone}"),
                if op == "team_auth_code" { 3 } else { 5 },
                900,
            )
            .await?;
        }
        _ => {}
    }
    conn.transaction::<Response, ApiError, _>(|c| {
        Box::pin(async move {
            // Lock only the affected session; constraints remain the final concurrency guard.
            if write {
                let mut event = params.get("eventId").cloned();
                if event.is_none() {
                    let entity = if op.starts_with("assignment_")
                        || matches!(
                            op,
                            "public_confirm" | "public_available" | "public_unavailable"
                        ) {
                        Some(Entity::Assignment)
                    } else if op.starts_with("zone_")
                        || op == "position_create"
                        || op == "position_order"
                    {
                        Some(Entity::Zone)
                    } else if op.starts_with("position_") {
                        Some(Entity::Position)
                    } else {
                        None
                    };
                    if let (Some(entity), Some(id)) = (entity, params.get("id")) {
                        let row = db::get(c, entity, id).await?;
                        event = match entity {
                            Entity::Assignment | Entity::Zone => {
                                row["eventId"].as_str().map(str::to_owned)
                            }
                            Entity::Position => {
                                let z = db::get(c, Entity::Zone, s(&row, "zoneId")).await?;
                                z["eventId"].as_str().map(str::to_owned)
                            }
                            _ => None,
                        };
                    }
                }
                let mut locks = event.into_iter().collect::<Vec<_>>();
                if op == "assignment_copy" {
                    locks.push(required(&body, "toEventId")?.to_owned());
                }
                if op == "roster_copy" {
                    locks.push(required(&body, "fromEventId")?.to_owned());
                }
                if op == "imports_commit" {
                    use diesel::sql_types::Text;
                    let rows = diesel::sql_query("SELECT data FROM roster_imports WHERE id=$1")
                        .bind::<Text, _>(param(&params, "id")?)
                        .load::<db::Document>(c)
                        .await?;
                    if let Some(row) = rows.as_slice().first() {
                        locks.push(s(&row.data, "eventId").to_owned());
                    }
                }
                if op.starts_with("member_") {
                    locks.push("member-directory".into());
                }
                locks.sort();
                locks.dedup();
                for event in locks {
                    diesel::sql_query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
                        .bind::<diesel::sql_types::Text, _>(event)
                        .execute(c)
                        .await?;
                }
            }
            let p = &params;
            let q = &query;
            let b = &body;
            let i = identity.as_ref();
            if op.starts_with("event_")
                || op.starts_with("zone_")
                || op.starts_with("position_")
                || op == "venue_template"
            {
                events::dispatch(c, op, p, q, b, &actor).await
            } else if op.starts_with("member_") {
                members::dispatch(c, op, p, q, b, &actor).await
            } else if op.starts_with("assignment_") || op.starts_with("roster_") {
                assignments::dispatch(c, op, p, q, b, &actor).await
            } else if public {
                public::dispatch(c, op, p, q, b).await
            } else {
                operations::dispatch(c, op, p, q, b, &actor, i).await
            }
        })
        .into()
    })
    .await
}
