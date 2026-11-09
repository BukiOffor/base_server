use super::*;
use diesel_async::AsyncPgConnection;
use svc::{events as service, queries::Roster};
pub async fn dispatch(
    c: &mut AsyncPgConnection,
    op: &str,
    p: &HashMap<String, String>,
    q: &ListQuery,
    b: &Value,
    actor: &str,
) -> Result<Response> {
    match op {
        "event_list" => {
            let mut es = db::list(c, Entity::Event).await?;
            es.retain(|e| {
                q.status
                    .as_ref()
                    .is_none_or(|s| s.split(',').any(|x| x == svc::s(e, "status")))
                    && q.q.as_ref().is_none_or(|q| {
                        format!("{} {}", s(e, "name"), s(e, "code"))
                            .to_lowercase()
                            .contains(&q.to_lowercase())
                    })
                    && q.series.as_ref().is_none_or(|x| s(e, "series") == x)
            });
            es.sort_by_key(|e| {
                (
                    s(e, "date").to_owned(),
                    if s(e, "session") == "morning" { 0 } else { 1 },
                )
            });
            for ev in &mut es {
                ev["stats"] = Roster::load(c, s(ev, "id")).await?.stats();
            }
            Ok(ok(json!(es)))
        }
        "event_create" => {
            let typed: crate::dto::roster::EventInput = serde_json::from_value(b.clone())?;
            Ok(created(
                service::create(c, serde_json::to_value(typed)?, actor).await?,
            ))
        }
        "event_get" => {
            let r = Roster::load(c, param(p, "eventId")?).await?;
            let mut event = r.event.clone();
            event["stats"] = r.stats();
            Ok(ok(event))
        }
        "event_patch" => {
            let mut ev = db::get(c, Entity::Event, param(p, "eventId")?).await?;
            svc::merge(&mut ev, b, service::FIELDS)?;
            service::validate(&ev)?;
            db::save(c, Entity::Event, &ev).await?;
            svc::activity(c, actor, "event", "updated event").await?;
            Ok(ok(ev))
        }
        "event_duplicate" => {
            let source = param(p, "eventId")?;
            let mut ev = db::get(c, Entity::Event, source).await?;
            ev["name"] = json!(format!("{} (Copy)", s(&ev, "name")));
            ev["code"] = json!(format!("{} COPY", s(&ev, "code")));
            ev["copyStructureFrom"] = json!(source);
            ev["status"] = json!("draft");
            Ok(created(service::create(c, ev, actor).await?))
        }
        "event_delete" => {
            let id = param(p, "eventId")?;
            let settings = db::get(c, Entity::Settings, "settings").await?;
            if s(&settings, "currentEventId") == id {
                return Err(ApiError::new(
                    422,
                    "CURRENT_EVENT",
                    "Cannot delete the current event",
                ));
            }
            db::delete(c, Entity::Event, id).await?;
            svc::activity(c, actor, "event", "deleted event").await?;
            Ok(empty())
        }
        "event_archive" | "event_publish" | "event_unpublish" => {
            let mut ev = db::get(c, Entity::Event, param(p, "eventId")?).await?;
            if op == "event_archive" {
                ev["status"] = json!("archived")
            } else {
                ev["rosterStatus"] = json!(if op == "event_publish" {
                    "published"
                } else {
                    "draft"
                });
                ev["publishedAt"] = if op == "event_publish" {
                    json!(svc::now())
                } else {
                    Value::Null
                };
            }
            ev["updatedAt"] = json!(svc::now());
            db::save(c, Entity::Event, &ev).await?;
            svc::activity(
                c,
                actor,
                if op == "event_archive" {
                    "event"
                } else {
                    "publish"
                },
                format!("{} {}", op, s(&ev, "name")),
            )
            .await?;
            if op == "event_publish" {
                svc::notify(c, "Roster published", s(&ev, "name"), "success").await?;
            }
            Ok(ok(ev))
        }
        "zone_list" => Ok(ok(json!(
            Roster::load(c, param(p, "eventId")?).await?.zones()
        ))),
        "zone_create" => {
            let z = service::add_zone(c, param(p, "eventId")?, b.clone(), true).await?;
            svc::activity(c, actor, "position", "added zone").await?;
            Ok(created(z))
        }
        "zone_patch" => {
            let mut z = db::get(c, Entity::Zone, param(p, "id")?).await?;
            svc::merge(&mut z, b, &["name", "mapRef", "order"])?;
            required(&z, "name")?;
            if !z["order"].is_null() && z["order"].as_u64().is_none() {
                return Err(ApiError::bad("order must be non-negative"));
            }
            if !service::spot_section(&z)
                && db::children(c, Entity::Position, s(&z, "id"))
                    .await?
                    .iter()
                    .any(|p| !p["spot"].is_null())
            {
                return Err(ApiError::new(
                    422,
                    "INVALID_SPOT",
                    "A section with usher seats must retain a valid seating map reference",
                ));
            }
            db::save(c, Entity::Zone, &z).await?;
            svc::activity(c, actor, "position", "updated zone").await?;
            Ok(ok(z))
        }
        "zone_delete" | "position_delete" => {
            db::delete(
                c,
                if op == "zone_delete" {
                    Entity::Zone
                } else {
                    Entity::Position
                },
                param(p, "id")?,
            )
            .await?;
            svc::activity(c, actor, "position", "deleted zone or position").await?;
            Ok(empty())
        }
        "position_create" => {
            let z = db::get(c, Entity::Zone, param(p, "id")?).await?;
            let v = service::add_position(c, &z, b.clone()).await?;
            svc::activity(c, actor, "position", "added position").await?;
            Ok(created(v))
        }
        "position_patch" => {
            let mut pos = db::get(c, Entity::Position, param(p, "id")?).await?;
            svc::merge(&mut pos, b, &["name", "required", "isLeadRole", "duty"])?;
            required(&pos, "name")?;
            svc::one_of(&pos, "duty", svc::queries::DUTIES)?;
            if pos["required"].as_u64().is_none() {
                return Err(ApiError::bad("required must be non-negative"));
            }
            if !pos["spot"].is_null() && pos["required"] != 1 {
                return Err(ApiError::new(
                    422,
                    "SPOT_CAPACITY",
                    "Spot headcount must remain one",
                ));
            }
            if !pos["isLeadRole"].is_boolean() {
                return Err(ApiError::bad("isLeadRole must be boolean"));
            }
            db::save(c, Entity::Position, &pos).await?;
            svc::activity(c, actor, "position", "updated position").await?;
            Ok(ok(pos))
        }
        "zone_order" | "position_order" => {
            let (entity, parent, key) = if op == "zone_order" {
                (Entity::Zone, param(p, "eventId")?, "zoneIds")
            } else {
                (Entity::Position, param(p, "id")?, "positionIds")
            };
            let ids = b[key]
                .as_array()
                .ok_or_else(|| ApiError::bad("Provide ordered IDs"))?;
            let rows = db::children(c, entity, parent).await?;
            if ids.len() != rows.len()
                || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
                || rows.iter().any(|v| !ids.contains(&v["id"]))
            {
                return Err(ApiError::bad("IDs must contain every child exactly once"));
            }
            for (mut row, i) in rows.into_iter().map(|row| {
                let i = ids.iter().position(|x| x == &row["id"]).unwrap();
                (row, i)
            }) {
                row["order"] = json!(i);
                db::save(c, entity, &row).await?;
            }
            svc::activity(c, actor, "position", "reordered structure").await?;
            Ok(empty())
        }
        "zone_template" => {
            if s(b, "mode") != "add-missing" {
                return Err(ApiError::bad("mode must be add-missing"));
            }
            let event = param(p, "eventId")?;
            service::apply_template(c, event).await?;
            svc::activity(c, actor, "position", "applied venue template").await?;
            Ok(ok(json!(Roster::load(c, event).await?.zones())))
        }
        "venue_template" => Ok(ok(service::template())),
        _ => Err(ApiError::missing()),
    }
}
