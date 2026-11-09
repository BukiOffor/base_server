use super::*;
use diesel_async::AsyncPgConnection;
use svc::queries::{DUTIES, Roster};
pub async fn dispatch(
    c: &mut AsyncPgConnection,
    op: &str,
    p: &HashMap<String, String>,
    q: &ListQuery,
    b: &Value,
    actor: &str,
    identity: Option<&admin::Identity>,
) -> Result<Response> {
    match op {
        "auth_login" => Ok(ok(
            admin::login(c, serde_json::from_value(b.clone())?).await?
        )),
        "auth_refresh" => {
            let r: crate::dto::roster::Refresh = serde_json::from_value(b.clone())?;
            Ok(ok(admin::refresh(c, &r.refresh_token).await?))
        }
        "auth_me" => Ok(ok(admin::admin(
            c,
            identity.ok_or_else(ApiError::unauthorized)?,
        )
        .await?)),
        "auth_logout" | "team_auth_logout" => {
            admin::revoke(c, &identity.ok_or_else(ApiError::unauthorized)?.sid).await?;
            Ok(empty())
        }
        "auth_forgot" => {
            admin::forgot(c, required(b, "email")?).await?;
            Ok(empty())
        }
        "auth_reset" => {
            admin::reset(c, required(b, "token")?, required(b, "password")?).await?;
            Ok(empty())
        }
        "team_auth_code" => {
            svc::team::request_code(c, required(b, "phone")?).await?;
            Ok(empty())
        }
        "team_auth_verify" => Ok(ok(svc::team::verify(
            c,
            required(b, "phone")?,
            required(b, "code")?,
        )
        .await?)),
        "team_me" => Ok(ok(svc::team::me(
            c,
            &identity.ok_or_else(ApiError::unauthorized)?.sub,
            q.event_id.as_deref(),
        )
        .await?)),
        "team_map" => {
            let me = db::get(
                c,
                Entity::Member,
                &identity.ok_or_else(ApiError::unauthorized)?.sub,
            )
            .await?;
            let members = db::list(c, Entity::Member).await?;
            let scope = svc::team::scope(&members, &me);
            let r = Roster::load(c, param(p, "eventId")?).await?;
            if !r
                .zones
                .iter()
                .any(|z| scope.iter().any(|n| n == s(z, "name")))
            {
                return Err(ApiError::forbidden());
            }
            let mut map = r.map(false, Some(&scope));
            if let Some(zones) = map["zones"].as_array_mut() {
                zones.retain(|z| scope.iter().any(|n| n == s(z, "name")));
                if r.event["rosterStatus"] != "published" {
                    for z in zones {
                        z.as_object_mut().unwrap().remove("stats");
                    }
                }
            }
            Ok(ok(map))
        }
        "team_tree" => Ok(ok(svc::team::tree(c).await?)),
        "zones_names" => {
            let settings = db::get(c, Entity::Settings, "settings").await?;
            let mut out = json!({"aisle":[],"section":[],"door":[],"area":[]});
            if let Some(current) = settings["currentEventId"].as_str() {
                let event = db::get(c, Entity::Event, current).await?;
                for event in svc::queries::sessions(c, &event).await? {
                    for z in db::children(c, Entity::Zone, s(&event, "id")).await? {
                        if let Some(a) = out[s(&z, "kind")].as_array_mut()
                            && !a.contains(&z["name"])
                        {
                            a.push(z["name"].clone());
                        }
                    }
                }
            }
            Ok(ok(out))
        }
        "map" => Ok(ok(Roster::load(c, param(p, "eventId")?)
            .await?
            .map(false, None))),
        "absences" => {
            let r = Roster::load(c, param(p, "eventId")?).await?;
            let rows: Vec<_> = r
                .absences()
                .into_iter()
                .filter(|a| {
                    q.covered.is_none_or(|v| a["covered"] == v)
                        && (q.zone_names.is_empty()
                            || q.zone_names
                                .iter()
                                .any(|name| name == s(&a["zone"], "name")))
                })
                .collect();
            Ok(ok(json!(rows)))
        }
        "overview" | "report_summary" => {
            let r = Roster::load(c, param(p, "eventId")?).await?;
            let by_duty = r.by("duty", DUTIES);
            if op == "overview" {
                let mut activity = db::list(c, Entity::Activity).await?;
                svc::sort(&mut activity, "at", true);
                activity.truncate(7);
                let mut sessions = svc::queries::sessions(c, &r.event).await?;
                for ev in &mut sessions {
                    ev["stats"] =
                        json!({"assigned":Roster::load(c,s(ev,"id")).await?.stats()["assigned"]});
                }
                let mut abs = r.absences();
                abs.truncate(8);
                let zones: Vec<_> = r
                    .zones
                    .iter()
                    .map(|z| {
                        let mut stat = r.zone_stat(z);
                        for key in ["name", "kind", "mapRef"] {
                            stat[key] = z[key].clone();
                        }
                        stat
                    })
                    .collect();
                Ok(ok(
                    json!({"event":r.event,"sessions":sessions,"stats":r.stats(),"zoneStats":zones,"byDuty":by_duty,"absences":abs,"activity":activity}),
                ))
            } else {
                let spots: Vec<_> = r
                    .positions
                    .iter()
                    .filter(|p| !p["spot"].is_null())
                    .collect();
                let filled = spots
                    .iter()
                    .filter(|p| {
                        r.assignments
                            .iter()
                            .any(|a| a["positionId"] == p["id"] && svc::active(a))
                    })
                    .count();
                let needs = spots
                    .iter()
                    .filter(|p| {
                        r.assignments
                            .iter()
                            .any(|a| a["positionId"] == p["id"] && !svc::active(a) && !r.covered(a))
                    })
                    .count();
                let mut lowest: Vec<_> = r.zones.iter().map(|z| r.zone_stat(z)).collect();
                lowest.sort_by(|a, b| {
                    a["coverage"]
                        .as_f64()
                        .unwrap_or(0.0)
                        .total_cmp(&b["coverage"].as_f64().unwrap_or(0.0))
                });
                lowest.truncate(5);
                Ok(ok(
                    json!({"stats":r.stats(),"byKind":r.by("kind",&["aisle","section","door","area"]),"byDuty":by_duty,"spots":{"total":spots.len(),"filled":filled,"needsCover":needs},"lowestCoverage":lowest}),
                ))
            }
        }
        "activity" => {
            let mut rows = db::list(c, Entity::Activity).await?;
            svc::sort(&mut rows, "at", true);
            rows.truncate(q.limit.unwrap_or(60).min(500));
            Ok(ok(json!(rows)))
        }
        "notifications" => {
            svc::notifications::periodic_notifications(c).await?;
            let mut rows = db::list(c, Entity::Notification).await?;
            svc::sort(&mut rows, "at", true);
            let unread = rows.iter().filter(|n| n["read"] == false).count();
            Ok(ok(json!({"items":rows,"unread":unread})))
        }
        "notifications_read_all" => {
            for mut n in db::list(c, Entity::Notification).await? {
                n["read"] = json!(true);
                db::save(c, Entity::Notification, &n).await?;
            }
            Ok(empty())
        }
        "notification_read" => {
            let mut n = db::get(c, Entity::Notification, param(p, "id")?).await?;
            n["read"] = json!(true);
            db::save(c, Entity::Notification, &n).await?;
            Ok(empty())
        }
        "settings_get" => {
            let mut v = db::get(c, Entity::Settings, "settings").await?;
            v.as_object_mut().unwrap().remove("id");
            Ok(ok(v))
        }
        "settings_patch" | "settings_current" => {
            let mut v = db::get(c, Entity::Settings, "settings").await?;
            if op == "settings_current" {
                v["currentEventId"] = json!(required(b, "eventId")?);
            } else {
                svc::merge(
                    &mut v,
                    b,
                    &[
                        "currentEventId",
                        "contactName",
                        "contactPhone",
                        "showTeammatePhones",
                        "allowSelfConfirm",
                    ],
                )?;
            }
            if let Some(event) = v["currentEventId"].as_str() {
                db::get(c, Entity::Event, event).await?;
            }
            if !s(&v, "contactPhone").is_empty() {
                v["contactPhone"] = json!(svc::normalize_phone(s(&v, "contactPhone"))?);
            }
            for key in ["showTeammatePhones", "allowSelfConfirm"] {
                if !v[key].is_boolean() {
                    return Err(ApiError::bad(format!("{key} must be boolean")));
                }
            }
            v["id"] = json!("settings");
            db::save(c, Entity::Settings, &v).await?;
            svc::activity(c, actor, "event", "updated settings").await?;
            v.as_object_mut().unwrap().remove("id");
            Ok(ok(v))
        }
        "search" => {
            let search = q.q.as_deref().unwrap_or("").to_lowercase();
            if search.trim().is_empty() {
                return Ok(ok(json!({"members":[],"zones":[],"events":[]})));
            }
            let mut members: Vec<_> = db::list(c, Entity::Member)
                .await?
                .into_iter()
                .filter(|m| {
                    format!(
                        "{} {} {} {}",
                        s(m, "firstName"),
                        s(m, "lastName"),
                        s(m, "phone"),
                        s(m, "memberId")
                    )
                    .to_lowercase()
                    .contains(&search)
                })
                .take(6)
                .collect();
            if let Some(event) = &q.event_id {
                let r = Roster::load(c, event).await?;
                for m in &mut members {
                    m["assignment"] = r
                        .assignments
                        .iter()
                        .find(|a| a["memberId"] == m["id"])
                        .map(|a| r.enriched(a))
                        .unwrap_or(Value::Null);
                }
            }
            let zones = if let Some(event) = &q.event_id {
                db::children(c, Entity::Zone, event).await?
            } else {
                db::list(c, Entity::Zone).await?
            };
            let zones: Vec<_> = zones
                .into_iter()
                .filter(|z| {
                    format!("{} {}", s(z, "name"), s(z, "mapRef"))
                        .to_lowercase()
                        .contains(&search)
                })
                .take(6)
                .collect();
            let events: Vec<_> = db::list(c, Entity::Event)
                .await?
                .into_iter()
                .filter(|e| {
                    format!("{} {}", s(e, "name"), s(e, "code"))
                        .to_lowercase()
                        .contains(&search)
                })
                .take(4)
                .collect();
            Ok(ok(json!({"members":members,"zones":zones,"events":events})))
        }
        "imports_template" => {
            let rows = vec![
                [
                    "first_name",
                    "last_name",
                    "phone",
                    "email",
                    "gender",
                    "unit",
                    "zone",
                    "position",
                ]
                .map(str::to_owned)
                .to_vec(),
            ];
            let f = q.format.as_deref().unwrap_or("csv");
            if !matches!(f, "csv" | "xlsx") {
                return Err(ApiError::bad("Template format must be csv or xlsx"));
            }
            download(svc::exports::file(
                &rows,
                f,
                "members-template".into(),
                "Member import template",
            )?)
        }
        "imports_commit" => Ok(ok(svc::imports::commit(c, param(p, "id")?, b, actor).await?)),
        "export" => download(
            svc::exports::export(
                c,
                param(p, "eventId")?,
                param(p, "kind")?,
                q.format.as_deref().unwrap_or("csv"),
                q.zone_id.as_deref(),
            )
            .await?,
        ),
        "admin_reset" => {
            if std::env::var("APP_ENV").as_deref() != Ok("development") {
                return Err(ApiError::missing());
            }
            let mut settings = db::get(c, Entity::Settings, "settings").await?;
            settings["currentEventId"] = Value::Null;
            settings["id"] = json!("settings");
            db::save(c, Entity::Settings, &settings).await?;
            for table in [
                "roster_events",
                "roster_members",
                "roster_activity",
                "roster_notifications",
                "roster_imports",
            ] {
                diesel::sql_query(format!("DELETE FROM {table}"))
                    .execute(c)
                    .await?;
            }
            let date = (chrono::Utc::now() + chrono::Duration::days(30))
                .format("%Y-%m-%d")
                .to_string();
            let ev=svc::events::create(c,json!({"name":"Demo Programme","code":"DEMO","date":date,"venue":"Koinonia Global Arena","reportingTime":"07:30","startTime":"08:45","status":"draft","useTemplate":true}),actor).await?;
            let mut seeded = vec![];
            for n in 1..=8 {
                seeded.push(svc::members::create_inner(c,json!({"firstName":format!("Demo{n}"),"lastName":"Usher","phone":format!("080200000{n:02}")}),actor,false).await?);
            }
            svc::members::role(c, s(&seeded[0], "id"), &json!({"role":"leader"}), actor).await?;
            svc::members::role(c,s(&seeded[1],"id"),&json!({"role":"coordinator","leaderId":seeded[0]["id"],"coordinates":["Section 41","Section 42"]}),actor).await?;
            let zones = db::children(c, Entity::Zone, s(&ev, "id")).await?;
            if let Some(zone) = zones.iter().find(|z| z["mapRef"] == "S41") {
                let mut positions = db::children(c, Entity::Position, s(zone, "id")).await?;
                positions.sort_by_key(|p| p["order"].as_u64().unwrap_or(0));
                for (member, position) in seeded.iter().skip(2).zip(positions.iter()) {
                    svc::assignments::assign(c,s(&ev,"id"),&json!({"memberId":member["id"],"zoneId":zone["id"],"positionId":position["id"]}),actor,false).await?;
                }
            }
            settings["currentEventId"] = ev["id"].clone();
            db::save(c, Entity::Settings, &settings).await?;
            Ok(ok(json!({"event":ev})))
        }
        _ => Err(ApiError::missing()),
    }
}
