use super::*;
use crate::auth::admin;
pub async fn request_code(c: &mut AsyncPgConnection, phone: &str) -> Result<()> {
    let phone = normalize_phone(phone)?;
    if std::env::var("AUTH_DELIVERY_URL").is_err() {
        return Err(ApiError::new(
            503,
            "DELIVERY_UNAVAILABLE",
            "Authentication delivery is not configured",
        ));
    }
    if let Some(m) = db::member_by_phone(c, &phone)
        .await?
        .filter(|m| matches!(s(m, "role"), "coordinator" | "leader"))
    {
        use rand::Rng;
        let code = format!("{:06}", rand::rng().random_range(0..1_000_000));
        admin::store(c, &format!("code:{phone}"), s(&m, "id"), "code", 600, &code).await?;
        if let Err(error) = admin::deliver("team-code", &phone, &code).await {
            admin::revoke(c, &format!("code:{phone}")).await?;
            tracing::warn!(code = error.code, "Team code delivery failed");
        }
    }
    Ok(())
}
pub async fn verify(c: &mut AsyncPgConnection, phone: &str, code: &str) -> Result<Value> {
    let phone = normalize_phone(phone)?;
    let key = format!("code:{phone}");
    if !admin::check_code(c, &key, code).await? {
        return Err(ApiError::new(401, "BAD_CODE", "Invalid or expired code"));
    }
    let m = db::member_by_phone(c, &phone)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    if !matches!(s(&m, "role"), "coordinator" | "leader") {
        return Err(ApiError::new(
            403,
            "NOT_TEAM",
            "This number isn't set up as a coordinator or leader",
        ));
    }
    admin::revoke(c, &key).await?;
    admin::team_issue(c, &m).await
}
pub fn coordinators<'a>(members: &'a [Value], me: &Value) -> Vec<&'a Value> {
    members
        .iter()
        .filter(|m| {
            if s(me, "role") == "leader" {
                m["leaderId"] == me["id"] && m["role"] == "coordinator"
            } else {
                m["id"] == me["id"] && m["role"] == "coordinator"
            }
        })
        .collect()
}
pub fn scope(members: &[Value], me: &Value) -> Vec<String> {
    coordinators(members, me)
        .into_iter()
        .flat_map(|m| m["coordinates"].as_array().cloned().unwrap_or_default())
        .filter_map(|n| n.as_str().map(str::to_owned))
        .collect()
}
pub async fn me(c: &mut AsyncPgConnection, member: &str, event: Option<&str>) -> Result<Value> {
    let me = db::get(c, Entity::Member, member).await?;
    if !matches!(s(&me, "role"), "coordinator" | "leader") {
        return Err(ApiError::forbidden());
    }
    let settings = db::get(c, Entity::Settings, "settings").await?;
    let eid = event
        .or(settings["currentEventId"].as_str())
        .ok_or_else(ApiError::missing)?;
    let r = queries::Roster::load(c, eid).await?;
    let members: Vec<_> = r.members.values().cloned().collect();
    let scope = scope(&members, &me);
    let published = r.event["rosterStatus"] == "published";
    let abs = r.absences();
    let mut coordinators_out = vec![];
    let mut zone_count = 0;
    let mut assigned = 0u64;
    let mut required = 0u64;
    let mut unavailable = 0;
    let mut needs = 0;
    for co in coordinators(&members, &me) {
        let mut zones = vec![];
        for z in r.zones.iter().filter(|z| {
            co["coordinates"]
                .as_array()
                .is_some_and(|ns| ns.contains(&z["name"]))
        }) {
            let mut stats = r.zone_stat(z);
            if !published {
                stats["assigned"] = json!(0);
                stats["coverage"] = json!(0);
                stats["lead"] = Value::Null;
            }
            let ushers:Vec<_>=r.assignments.iter().filter(|a|published&&active(a)&&a["zoneId"]==z["id"]).map(|a|json!({"assignmentId":a["id"],"member":brief(&r.member(a),true),"positionId":a["positionId"],"status":a["status"],"isTeamLead":a["isTeamLead"]})).collect();
            let away: Vec<_> = abs
                .iter()
                .filter(|a| published && a["zone"]["id"] == z["id"])
                .cloned()
                .collect();
            zones.push(json!({"zone":z,"stats":stats,"positions":r.positions.iter().filter(|p|p["zoneId"]==z["id"]).collect::<Vec<_>>(),"ushers":ushers,"unavailable":away}));
        }
        coordinators_out.push(json!({"member":brief(co,true),"zones":zones}));
    }
    for z in r
        .zones
        .iter()
        .filter(|z| scope.contains(&s(z, "name").to_owned()))
    {
        zone_count += 1;
        let stat = r.zone_stat(z);
        required += stat["required"].as_u64().unwrap_or(0);
        if published {
            assigned += stat["assigned"].as_u64().unwrap_or(0);
            for a in abs.iter().filter(|a| a["zone"]["id"] == z["id"]) {
                unavailable += 1;
                if a["covered"] == false {
                    needs += 1;
                }
            }
        }
    }
    let mut brief_me = brief(&me, true);
    brief_me["role"] = me["role"].clone();
    if let Some(leader) = members.iter().find(|m| m["id"] == me["leaderId"]) {
        brief_me["leader"] = brief(leader, true);
    }
    let sessions = queries::sessions(c, &r.event).await?;
    let mut notifications: Vec<_> = db::list(c, Entity::Notification)
        .await?
        .into_iter()
        .filter(|n| {
            n["recipientIds"]
                .as_array()
                .is_some_and(|ids| ids.contains(&me["id"]))
        })
        .collect();
    sort(&mut notifications, "at", true);
    notifications.truncate(30);
    Ok(
        json!({"me":brief_me,"event":r.event,"rosterStatus":if published{"published"}else{"draft"},"sessions":sessions,"coordinators":coordinators_out,"notifications":notifications,"stats":{"zones":zone_count,"assigned":assigned,"required":required,"unavailable":unavailable,"needsCover":needs}}),
    )
}
pub async fn tree(c: &mut AsyncPgConnection) -> Result<Value> {
    let members = db::list(c, Entity::Member).await?;
    let settings = db::get(c, Entity::Settings, "settings").await?;
    let coords: Vec<_> = members
        .iter()
        .filter(|m| m["role"] == "coordinator")
        .collect();
    let leaders:Vec<_>=members.iter().filter(|m|m["role"]=="leader").map(|m|json!({"member":m,"coordinators":coords.iter().filter(|co|co["leaderId"]==m["id"]).map(|co|json!({"member":co,"coordinates":co["coordinates"]})).collect::<Vec<_>>()})).collect();
    let unassigned: Vec<_> = coords
        .iter()
        .filter(|co| co["leaderId"].is_null())
        .map(|co| json!({"member":co,"coordinates":co["coordinates"]}))
        .collect();
    let zones = if let Some(id) = settings["currentEventId"].as_str() {
        db::children(c, Entity::Zone, id).await?
    } else {
        vec![]
    };
    let uncovered: Vec<_> = zones
        .iter()
        .filter(|z| {
            !coords.iter().any(|co| {
                co["coordinates"]
                    .as_array()
                    .is_some_and(|ns| ns.contains(&z["name"]))
            })
        })
        .map(|z| z["name"].clone())
        .collect();
    Ok(json!({"leaders":leaders,"unassignedCoordinators":unassigned,"uncoveredZones":uncovered}))
}
