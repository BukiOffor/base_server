use super::*;
pub async fn assign(
    c: &mut AsyncPgConnection,
    event: &str,
    p: &Value,
    actor: &str,
    log: bool,
) -> Result<Value> {
    let member = required(p, "memberId")?;
    db::get(c, Entity::Member, member).await?;
    let zone = db::get(c, Entity::Zone, required(p, "zoneId")?).await?;
    let pos = db::get(c, Entity::Position, required(p, "positionId")?).await?;
    if s(&zone, "eventId") != event || pos["zoneId"] != zone["id"] {
        return Err(ApiError::new(
            422,
            "INVALID_POSITION",
            "Position and zone must belong to this session",
        ));
    }
    let spot = !pos["spot"].is_null();
    let assignments = db::children(c, Entity::Assignment, event).await?;
    let previous = assignments
        .iter()
        .find(|a| s(a, "memberId") == member)
        .cloned();
    let displaced = if spot {
        assignments
            .iter()
            .find(|a| a["positionId"] == pos["id"] && active(a) && s(a, "memberId") != member)
            .cloned()
    } else {
        None
    };
    if displaced.is_some() && p["replace"] == false {
        return Err(ApiError::new(
            409,
            "SPOT_TAKEN",
            "This seat is already occupied",
        ));
    }
    if let Some(a) = &previous {
        db::delete(c, Entity::Assignment, s(a, "id")).await?;
    }
    if let Some(a) = &displaced {
        db::delete(c, Entity::Assignment, s(a, "id")).await?;
    }
    let lead = p["isTeamLead"]
        .as_bool()
        .unwrap_or(pos["isLeadRole"] == true);
    if lead {
        for mut a in assignments
            .into_iter()
            .filter(|a| a["zoneId"] == zone["id"] && a["isTeamLead"] == true)
        {
            if previous.as_ref().is_some_and(|x| x["id"] == a["id"])
                || displaced.as_ref().is_some_and(|x| x["id"] == a["id"])
            {
                continue;
            }
            a["isTeamLead"] = json!(false);
            db::save(c, Entity::Assignment, &a).await?;
        }
    }
    let a = json!({"id":id(),"memberId":member,"eventId":event,"zoneId":zone["id"],"positionId":pos["id"],"isTeamLead":lead,"status":"assigned","updatedAt":now(),"_isSpot":spot});
    db::save(c, Entity::Assignment, &a).await?;
    let mut replaced = json!({});
    if let Some(a) = previous {
        replaced["previousOfMember"] = clean(a);
    }
    if let Some(a) = displaced {
        let m = db::get(c, Entity::Member, s(&a, "memberId")).await?;
        replaced["displacedFromSpot"] = json!({"assignment":clean(a),"member":brief(&m,true)});
    }
    if log {
        activity(
            c,
            actor,
            "assign",
            format!("assigned usher to {}", s(&zone, "name")),
        )
        .await?;
    }
    Ok(json!({"assignment":clean(a),"replaced":replaced}))
}
pub async fn status(
    c: &mut AsyncPgConnection,
    id: &str,
    status: &str,
    reason: Option<&str>,
    actor: &str,
) -> Result<Value> {
    let mut a = db::get(c, Entity::Assignment, id).await?;
    let was_unavailable = !active(&a);
    let pos = db::get(c, Entity::Position, s(&a, "positionId")).await?;
    match status {
        "unavailable" => {
            if !active(&a) {
                return Err(ApiError::new(
                    409,
                    "ALREADY_UNAVAILABLE",
                    "Assignment is already unavailable",
                ));
            }
            if reason.is_some_and(|r| r.chars().count() > 200) {
                return Err(ApiError::bad("Reason must be at most 200 characters"));
            }
            let ev = db::get(c, Entity::Event, s(&a, "eventId")).await?;
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(
                &format!(
                    "{} {}",
                    s(&ev, "date"),
                    ev["startTime"].as_str().unwrap_or("23:59")
                ),
                "%Y-%m-%d %H:%M",
            ) {
                let nigeria = chrono::Utc::now().naive_utc() + chrono::Duration::hours(1);
                if dt <= nigeria {
                    return Err(ApiError::new(
                        422,
                        "SESSION_STARTED",
                        "This session has already started",
                    ));
                }
            }
            a["unavailableReason"] = json!(reason);
            a["unavailableAt"] = json!(now());
            a["isTeamLead"] = json!(false);
            let zone = db::get(c, Entity::Zone, s(&a, "zoneId")).await?;
            let mut recipients = std::collections::BTreeSet::new();
            for coordinator in db::list(c, Entity::Member).await?.iter().filter(|m| {
                m["role"] == "coordinator"
                    && m["coordinates"]
                        .as_array()
                        .is_some_and(|names| names.contains(&zone["name"]))
            }) {
                recipients.insert(s(coordinator, "id").to_owned());
                if let Some(leader) = coordinator["leaderId"].as_str() {
                    recipients.insert(leader.to_owned());
                }
            }
            db::save(c,Entity::Notification,&json!({"id":super::id(),"title":"Spot needs cover","body":format!("An usher cannot make {} ({})",s(&ev,"name"),s(&zone,"name")),"at":now(),"tone":"warn","read":false,"recipientIds":recipients})).await?;
        }
        "assigned" | "confirmed" => {
            if !active(&a) && !pos["spot"].is_null() {
                let assignments = db::children(c, Entity::Assignment, s(&a, "eventId")).await?;
                if assignments
                    .iter()
                    .any(|x| x["id"] != a["id"] && x["positionId"] == pos["id"] && active(x))
                {
                    return Err(ApiError::new(
                        409,
                        "SPOT_COVERED",
                        "Someone else now holds this seat",
                    ));
                }
            }
            a["unavailableReason"] = Value::Null;
            a["unavailableAt"] = Value::Null;
        }
        _ => return Err(ApiError::bad("Invalid assignment status")),
    }
    a["status"] = json!(status);
    a["updatedAt"] = json!(now());
    db::save(c, Entity::Assignment, &a).await?;
    activity(
        c,
        actor,
        if status == "unavailable" || was_unavailable {
            "absence"
        } else {
            "assign"
        },
        format!("marked assignment {status}"),
    )
    .await?;
    Ok(clean(a))
}
pub async fn patch(c: &mut AsyncPgConnection, id: &str, p: &Value, actor: &str) -> Result<Value> {
    let old = db::get(c, Entity::Assignment, id).await?;
    let mut merged = old.clone();
    merge(
        &mut merged,
        p,
        &[
            "zoneId",
            "positionId",
            "isTeamLead",
            "status",
            "unavailableReason",
        ],
    )?;
    let moving = old["positionId"] != merged["positionId"] || old["zoneId"] != merged["zoneId"];
    let lead_changed = old["isTeamLead"] != merged["isTeamLead"];
    if moving || lead_changed {
        if merged["status"] == "unavailable" && merged["isTeamLead"] == true {
            return Err(ApiError::new(
                422,
                "UNAVAILABLE_LEAD",
                "An unavailable usher cannot be team lead",
            ));
        }
        let result = assign(c, s(&old, "eventId"), &merged, actor, false).await?;
        let new_id = s(&result["assignment"], "id");
        let mut a = db::get(c, Entity::Assignment, new_id).await?;
        db::delete(c, Entity::Assignment, new_id).await?;
        a["id"] = json!(id);
        db::save(c, Entity::Assignment, &a).await?;
        if s(&merged, "status") != "assigned" {
            return status(
                c,
                id,
                s(&merged, "status"),
                merged["unavailableReason"].as_str(),
                actor,
            )
            .await;
        }
        activity(c, actor, "assign", "updated assignment").await?;
        return Ok(clean(a));
    }
    if p["status"].is_null() {
        return Ok(clean(old));
    }
    status(
        c,
        id,
        required(p, "status")?,
        p["unavailableReason"].as_str(),
        actor,
    )
    .await
}
pub async fn copy(
    c: &mut AsyncPgConnection,
    from: &str,
    to: &str,
    member_ids: Option<&Vec<Value>>,
    actor: &str,
) -> Result<Value> {
    if from == to {
        return Err(ApiError::bad("Source and destination must differ"));
    }
    db::get(c, Entity::Event, from).await?;
    db::get(c, Entity::Event, to).await?;
    let source_zones = db::children(c, Entity::Zone, from).await?;
    let dest_zones = db::children(c, Entity::Zone, to).await?;
    let mut copied = 0;
    let mut unmatched = vec![];
    for a in db::children(c, Entity::Assignment, from)
        .await?
        .into_iter()
        .filter(active)
    {
        if member_ids.is_some_and(|ids| !ids.contains(&a["memberId"])) {
            continue;
        }
        let sz = source_zones.iter().find(|z| z["id"] == a["zoneId"]);
        let sp = db::get(c, Entity::Position, s(&a, "positionId")).await?;
        let dz = sz.and_then(|sz| dest_zones.iter().find(|z| z["name"] == sz["name"]));
        if let Some(z) = dz {
            let ps = db::children(c, Entity::Position, s(z, "id")).await?;
            if let Some(p) = ps.iter().find(|p| p["name"] == sp["name"]) {
                assign(c,to,&json!({"memberId":a["memberId"],"zoneId":z["id"],"positionId":p["id"],"isTeamLead":a["isTeamLead"]}),actor,false).await?;
                copied += 1;
                continue;
            }
        }
        unmatched.push(json!({"memberId":a["memberId"],"reason":"Zone or position not found"}));
    }
    activity(
        c,
        actor,
        "assign",
        format!("copied {copied} assignments between sessions"),
    )
    .await?;
    Ok(json!({"copied":copied,"unmatched":unmatched}))
}
