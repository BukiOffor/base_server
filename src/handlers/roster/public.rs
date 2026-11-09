use super::*;
use diesel_async::AsyncPgConnection;
pub async fn dispatch(
    c: &mut AsyncPgConnection,
    op: &str,
    p: &HashMap<String, String>,
    _q: &ListQuery,
    b: &Value,
) -> Result<Response> {
    let settings = db::get(c, Entity::Settings, "settings").await?;
    match op {
        "public_settings" => {
            let mut v = settings;
            v.as_object_mut().unwrap().remove("id");
            v.as_object_mut().unwrap().remove("currentEventId");
            v.as_object_mut().unwrap().remove("updatedAt");
            Ok(ok(v))
        }
        "public_current" => {
            let Some(id) = settings["currentEventId"].as_str() else {
                return Ok(ok(json!({"event":null,"sessions":[],"stats":{}})));
            };
            let r = svc::queries::Roster::load(c, id).await?;
            let sessions = svc::queries::sessions(c, &r.event).await?;
            let stats = if r.event["rosterStatus"] == "published" {
                json!({"assigned":r.stats()["assigned"]})
            } else {
                json!({})
            };
            Ok(ok(
                json!({"event":r.event,"sessions":sessions,"stats":stats}),
            ))
        }
        "public_lookup" => {
            let typed: crate::dto::roster::Phone = serde_json::from_value(b.clone())?;
            Ok(ok(svc::queries::lookup(
                c,
                &typed.phone,
                typed.event_id.as_deref(),
            )
            .await?))
        }
        "public_map" => {
            let r = svc::queries::Roster::load(c, param(p, "eventId")?).await?;
            if r.event["rosterStatus"] != "published" {
                return Err(ApiError::missing());
            }
            Ok(ok(r.map(true, None)))
        }
        "public_confirm" | "public_unavailable" | "public_available" => {
            let aid = param(p, "id")?;
            let a = db::get(c, Entity::Assignment, aid).await?;
            let m = db::get(c, Entity::Member, s(&a, "memberId")).await?;
            if svc::normalize_phone(required(b, "phone")?)? != s(&m, "phone") {
                return Err(ApiError::forbidden());
            }
            let ev = db::get(c, Entity::Event, s(&a, "eventId")).await?;
            if ev["rosterStatus"] != "published" {
                return Err(ApiError::missing());
            }
            if op == "public_confirm" && settings["allowSelfConfirm"] != true {
                return Err(ApiError::forbidden());
            }
            if op == "public_confirm" && !svc::active(&a) {
                return Err(ApiError::new(
                    422,
                    "UNAVAILABLE_ASSIGNMENT",
                    "Restore availability before confirming",
                ));
            }
            let status = match op {
                "public_confirm" => "confirmed",
                "public_unavailable" => "unavailable",
                _ => "assigned",
            };
            let actor = format!("{} {}", s(&m, "firstName"), s(&m, "lastName"));
            let assignment =
                svc::assignments::status(c, aid, status, b["reason"].as_str(), &actor).await?;
            Ok(ok(json!({"assignment":assignment})))
        }
        _ => Err(ApiError::missing()),
    }
}
