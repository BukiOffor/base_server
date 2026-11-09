//! Shared roster validation and transactional domain services.
pub mod assignments;
pub mod events;
pub mod exports;
pub mod imports;
pub mod members;
pub mod notifications;
mod pdf;
pub mod queries;
pub mod team;
use crate::{
    errors::api::{ApiError, Result},
    models::roster::{self as db, Entity},
};
use diesel_async::AsyncPgConnection;
use serde_json::{Value, json};
pub fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}
pub fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
pub fn required<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| ApiError::bad(format!("{k} is required")))
}
pub fn one_of(v: &Value, key: &str, values: &[&str]) -> Result<()> {
    if !v[key].is_null() && !values.contains(&s(v, key)) {
        return Err(ApiError::bad(format!("Invalid {key}")));
    }
    Ok(())
}
pub fn normalize_phone(input: &str) -> Result<String> {
    let mut d: String = input.chars().filter(char::is_ascii_digit).collect();
    if d.len() == 13 && d.starts_with("234") {
        d = format!("0{}", &d[3..]);
    } else if d.len() == 10 && matches!(d.as_bytes()[0], b'7' | b'8' | b'9') {
        d.insert(0, '0');
    }
    let b = d.as_bytes();
    if b.len() != 11
        || b[0] != b'0'
        || !matches!(b[1], b'7' | b'8' | b'9')
        || !matches!(b[2], b'0' | b'1')
    {
        return Err(ApiError::bad("Enter a valid Nigerian phone number"));
    }
    Ok(d)
}
pub fn merge(v: &mut Value, patch: &Value, allowed: &[&str]) -> Result<()> {
    let obj = patch
        .as_object()
        .ok_or_else(|| ApiError::bad("Expected an object"))?;
    for (k, val) in obj {
        if !allowed.contains(&k.as_str()) {
            return Err(ApiError::bad(format!("Unknown or read-only field: {k}")));
        }
        v[k] = val.clone();
    }
    v["updatedAt"] = json!(now());
    Ok(())
}
pub fn stamp(v: &mut Value) {
    v["id"] = json!(id());
    v["createdAt"] = json!(now());
    v["updatedAt"] = json!(now());
}
pub fn clean(mut v: Value) -> Value {
    if let Some(o) = v.as_object_mut() {
        o.remove("_isSpot");
    }
    v
}
pub fn active(a: &Value) -> bool {
    s(a, "status") != "unavailable"
}
pub async fn activity(
    c: &mut AsyncPgConnection,
    actor: &str,
    kind: &str,
    text: impl Into<String>,
) -> Result<()> {
    db::save(c,Entity::Activity,&json!({"id":id(),"actor":actor.split_whitespace().next().unwrap_or(actor),"kind":kind,"text":text.into(),"at":now()})).await
}
pub async fn notify(c: &mut AsyncPgConnection, title: &str, body: &str, tone: &str) -> Result<()> {
    db::save(
        c,
        Entity::Notification,
        &json!({"id":id(),"title":title,"body":body,"tone":tone,"read":false,"at":now()}),
    )
    .await
}
pub fn paginate(mut items: Vec<Value>, q: &crate::dto::roster::ListQuery) -> Result<Value> {
    let page = q.page.unwrap_or(1);
    let size = q.page_size.unwrap_or(25).min(200);
    if page == 0 || size == 0 {
        return Err(ApiError::bad("page and pageSize must be positive"));
    }
    let total = items.len();
    let start = page.saturating_sub(1).saturating_mul(size).min(total);
    items = items.into_iter().skip(start).take(size).collect();
    Ok(json!({"items":items,"page":page,"pageSize":size,"total":total}))
}
pub fn sort(items: &mut [Value], field: &str, desc: bool) {
    items.sort_by(|a, b| {
        if let (Some(av), Some(bv)) = (a[field].as_i64(), b[field].as_i64()) {
            return if desc { bv.cmp(&av) } else { av.cmp(&bv) };
        }
        let av = if field == "name" {
            format!("{} {}", s(a, "firstName"), s(a, "lastName"))
        } else {
            a[field]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| a[field].to_string())
        };
        let bv = if field == "name" {
            format!("{} {}", s(b, "firstName"), s(b, "lastName"))
        } else {
            b[field]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| b[field].to_string())
        };
        if desc { bv.cmp(&av) } else { av.cmp(&bv) }
    })
}
pub fn brief(m: &Value, phone: bool) -> Value {
    let mut v = json!({"id":m["id"],"firstName":m["firstName"],"lastName":m["lastName"]});
    if phone {
        v["phone"] = m["phone"].clone();
    }
    v
}
/// Periodic notices and bounded retention for expired authentication artifacts.
pub fn start_maintenance(state: std::sync::Arc<crate::AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(900));
        loop {
            interval.tick().await;
            let Ok(mut c) = state.pool.get().await else {
                continue;
            };
            let result = async {
                use diesel_async::RunQueryDsl;
                for table in ["roster_tokens", "roster_imports", "roster_rate_limits"] {
                    diesel::sql_query(format!(
                        "DELETE FROM {table} WHERE expires_at<now()-interval '1 day'"
                    ))
                    .execute(&mut c)
                    .await?;
                }
                notifications::periodic_notifications(&mut c).await
            }
            .await;
            if let Err(error) = result {
                tracing::error!(code = error.code, "Roster maintenance failed");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phones() {
        for input in [
            "+234 803-123-4567",
            "2348031234567",
            "8031234567",
            "08031234567",
        ] {
            assert_eq!(normalize_phone(input).unwrap(), "08031234567");
        }
        for input in ["", "123", "06031234567", "08231234567", "080312345678"] {
            assert!(normalize_phone(input).is_err());
        }
    }
}
