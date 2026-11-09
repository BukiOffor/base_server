use super::*;
use diesel_async::AsyncPgConnection;
use svc::{assignments as service, queries::Roster};
pub async fn dispatch(
    c: &mut AsyncPgConnection,
    op: &str,
    p: &HashMap<String, String>,
    q: &ListQuery,
    b: &Value,
    actor: &str,
) -> Result<Response> {
    match op {
        "assignment_list" => {
            let r = Roster::load(c, param(p, "eventId")?).await?;
            let mut rows: Vec<_> = r
                .assignments
                .iter()
                .map(|a| r.enriched(a))
                .filter(|a| {
                    q.status.as_ref().is_none_or(|v| s(a, "status") == v)
                        && q.zone_id.as_ref().is_none_or(|v| s(a, "zoneId") == v)
                        && q.position_id
                            .as_ref()
                            .is_none_or(|v| s(a, "positionId") == v)
                        && q.duty.as_ref().is_none_or(|v| s(a, "duty") == v)
                        && q.q.as_ref().is_none_or(|q| {
                            format!(
                                "{} {} {}",
                                s(&a["member"], "firstName"),
                                s(&a["member"], "lastName"),
                                s(&a["member"], "phone")
                            )
                            .to_lowercase()
                            .contains(&q.to_lowercase())
                        })
                })
                .collect();
            for a in &mut rows {
                a["_member"] = json!(format!(
                    "{} {}",
                    s(&a["member"], "firstName"),
                    s(&a["member"], "lastName")
                ));
            }
            let key = match q.sort.as_deref().unwrap_or("zone") {
                "member" => "_member",
                "zone" => "zoneName",
                "position" => "positionName",
                "status" => "status",
                _ => return Err(ApiError::bad("Invalid assignment sort")),
            };
            svc::sort(&mut rows, key, q.dir.as_deref() == Some("desc"));
            for a in &mut rows {
                a.as_object_mut().unwrap().remove("_member");
            }
            Ok(ok(svc::paginate(rows, q)?))
        }
        "assignment_create" => {
            let typed: crate::dto::roster::AssignmentInput = serde_json::from_value(b.clone())?;
            Ok(created(
                service::assign(
                    c,
                    param(p, "eventId")?,
                    &serde_json::to_value(typed)?,
                    actor,
                    true,
                )
                .await?,
            ))
        }
        "assignment_patch" => Ok(ok(
            json!({"assignment":service::patch(c,param(p,"id")?,b,actor).await?}),
        )),
        "assignment_delete" => {
            db::delete(c, Entity::Assignment, param(p, "id")?).await?;
            svc::activity(c, actor, "assign", "removed assignment").await?;
            Ok(empty())
        }
        "assignment_confirm" => Ok(ok(
            json!({"assignment":service::status(c,param(p,"id")?,"confirmed",None,actor).await?}),
        )),
        "assignment_bulk" => {
            let event = param(p, "eventId")?;
            let ids = b["memberIds"]
                .as_array()
                .ok_or_else(|| ApiError::bad("memberIds must be an array"))?;
            let pos = db::get(c, Entity::Position, required(b, "positionId")?).await?;
            if !pos["spot"].is_null() && ids.len() > 1 {
                return Err(ApiError::new(
                    422,
                    "SPOT_CAPACITY",
                    "Only one member can be assigned to a spot",
                ));
            }
            let mut count = 0;
            let mut skipped = vec![];
            for mid in ids {
                let mid = mid
                    .as_str()
                    .ok_or_else(|| ApiError::bad("Invalid member ID"))?;
                if db::get(c, Entity::Member, mid).await.is_err() {
                    skipped.push(json!({"memberId":mid,"reason":"Member not found"}));
                    continue;
                }
                service::assign(
                    c,
                    event,
                    &json!({"memberId":mid,"zoneId":b["zoneId"],"positionId":b["positionId"]}),
                    actor,
                    false,
                )
                .await?;
                count += 1;
            }
            svc::activity(c, actor, "assign", format!("assigned {count} ushers")).await?;
            Ok(ok(json!({"assigned":count,"skipped":skipped})))
        }
        "assignment_bulk_remove" => {
            let event = param(p, "eventId")?;
            let ids = b["memberIds"]
                .as_array()
                .ok_or_else(|| ApiError::bad("memberIds must be an array"))?;
            let mut removed = 0;
            for a in db::children(c, Entity::Assignment, event).await? {
                if ids.contains(&a["memberId"]) {
                    db::delete(c, Entity::Assignment, s(&a, "id")).await?;
                    removed += 1;
                }
            }
            svc::activity(c, actor, "assign", format!("removed {removed} assignments")).await?;
            Ok(ok(json!({"removed":removed})))
        }
        "assignment_copy" => Ok(ok(service::copy(
            c,
            param(p, "eventId")?,
            required(b, "toEventId")?,
            b["memberIds"].as_array(),
            actor,
        )
        .await?)),
        "roster_copy" => {
            let to = param(p, "eventId")?;
            let ev = db::get(c, Entity::Event, to).await?;
            if ev["rosterStatus"] == "published" {
                return Err(ApiError::new(
                    422,
                    "ROSTER_PUBLISHED",
                    "Unpublish the destination roster first",
                ));
            }
            Ok(ok(service::copy(
                c,
                required(b, "fromEventId")?,
                to,
                None,
                actor,
            )
            .await?))
        }
        _ => Err(ApiError::missing()),
    }
}
