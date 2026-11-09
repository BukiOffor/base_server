use super::*;
use diesel_async::AsyncPgConnection;
pub async fn dispatch(
    c: &mut AsyncPgConnection,
    op: &str,
    p: &HashMap<String, String>,
    q: &ListQuery,
    b: &Value,
    actor: &str,
) -> Result<Response> {
    match op {
        "member_create" => {
            let typed: crate::dto::roster::MemberInput = serde_json::from_value(b.clone())?;
            Ok(created(
                svc::members::create(c, serde_json::to_value(typed)?, actor).await?,
            ))
        }
        "member_patch" => Ok(ok(svc::members::update(c, param(p, "id")?, b, actor).await?)),
        "member_role" => Ok(ok(svc::members::role(c, param(p, "id")?, b, actor).await?)),
        "member_check" => {
            let phone = svc::normalize_phone(
                q.phone
                    .as_deref()
                    .ok_or_else(|| ApiError::bad("phone is required"))?,
            )?;
            let member = db::member_by_phone(c, &phone).await?;
            Ok(ok(
                json!({"available":member.is_none(),"member":member.map(|m|json!({"id":m["id"],"name":format!("{} {}",s(&m,"firstName"),s(&m,"lastName"))}))}),
            ))
        }
        "member_delete" => {
            let id = param(p, "id")?;
            unlink_leader(c, id).await?;
            db::delete(c, Entity::Member, id).await?;
            svc::activity(c, actor, "member", "deleted member").await?;
            Ok(empty())
        }
        "member_bulk_delete" => {
            let ids = b["memberIds"]
                .as_array()
                .ok_or_else(|| ApiError::bad("memberIds must be an array"))?;
            let mut deleted = 0;
            for id in ids {
                let id = id
                    .as_str()
                    .ok_or_else(|| ApiError::bad("Invalid member id"))?;
                if db::get(c, Entity::Member, id).await.is_ok() {
                    unlink_leader(c, id).await?;
                    db::delete(c, Entity::Member, id).await?;
                    deleted += 1;
                }
            }
            svc::activity(c, actor, "member", format!("deleted {deleted} members")).await?;
            Ok(ok(json!({"deleted":deleted})))
        }
        "member_get" => {
            let mut m = db::get(c, Entity::Member, param(p, "id")?).await?;
            let assignments = db::member_assignments(c, param(p, "id")?).await?;
            m["assignments"] = json!(assignments);
            Ok(ok(m))
        }
        "member_list" => {
            if let Some(event) = &q.event_id {
                db::get(c, Entity::Event, event).await?;
            }
            Ok(ok(db::page_members(c, q).await?))
        }
        _ => Err(ApiError::missing()),
    }
}
async fn unlink_leader(c: &mut AsyncPgConnection, id: &str) -> Result<()> {
    for mut m in db::list(c, Entity::Member).await? {
        if s(&m, "leaderId") == id {
            m["leaderId"] = Value::Null;
            db::save(c, Entity::Member, &m).await?;
        }
    }
    Ok(())
}
