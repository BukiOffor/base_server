use super::*;
pub const FIELDS: &[&str] = &[
    "name",
    "code",
    "series",
    "day",
    "session",
    "theme",
    "description",
    "date",
    "venue",
    "reportingTime",
    "briefingTime",
    "startTime",
    "dressCode",
    "status",
    "instructions",
];
pub fn validate(v: &Value) -> Result<()> {
    for k in ["name", "code", "date", "venue", "reportingTime"] {
        required(v, k)?;
    }
    chrono::NaiveDate::parse_from_str(s(v, "date"), "%Y-%m-%d")
        .map_err(|_| ApiError::bad("date must be YYYY-MM-DD"))?;
    for k in ["reportingTime", "briefingTime", "startTime"] {
        if !v[k].is_null() {
            let t = s(v, k);
            if t.len() != 5 || chrono::NaiveTime::parse_from_str(t, "%H:%M").is_err() {
                return Err(ApiError::bad(format!("{k} must be HH:mm")));
            }
        }
    }
    one_of(v, "status", &["draft", "active", "completed", "archived"])?;
    one_of(v, "session", &["morning", "evening"])?;
    if !v["day"].is_null() && v["day"].as_u64().is_none_or(|d| d == 0) {
        return Err(ApiError::bad("day must be positive"));
    }
    Ok(())
}
pub fn spot_section(v: &Value) -> bool {
    s(v, "kind") == "section"
        && s(v, "mapRef")
            .strip_prefix('S')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=83).contains(&n) && ![5, 6, 7, 17, 18].contains(&n))
}
pub fn map_ref(name: &str) -> Option<String> {
    if let Some(n) = name
        .strip_prefix("Section ")
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=83).contains(n))
    {
        Some(format!("S{n}"))
    } else if let Some(n) = name
        .strip_prefix("Aisle ")
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=9).contains(n))
    {
        Some(format!("A{n}"))
    } else {
        name.strip_prefix("Door ").map(str::to_owned)
    }
}
fn position(name: &str, required: u32, duty: &str) -> Value {
    json!({"name":name,"required":required,"duty":duty,"isLeadRole":duty=="lead"})
}
pub fn seats() -> Vec<Value> {
    [
        ("FL", "Front row, left end"),
        ("FR", "Front row, right end"),
        ("BL", "Back row, left end"),
        ("BR", "Back row, right end"),
    ]
    .into_iter()
    .map(|(spot, name)| {
        let mut p = position(name, 1, "seat");
        p["spot"] = json!(spot);
        p
    })
    .collect()
}
pub fn template() -> Value {
    let mut zones = Vec::new();
    for n in 1..=9 {
        zones.push(json!({"name":format!("Aisle {n}"),"kind":"aisle","mapRef":format!("A{n}"),"positions":[position("Aisle Lead",1,"lead"),position("Crowd Control",6,"crowd"),position("Floor Marshal",3,"crowd")]}));
    }
    for n in 1..=83 {
        let ps = match n {
            5 | 7 => vec![position("Section Usher", 2, "protocol")],
            6 => vec![position("Protocol Support", 3, "protocol")],
            17 => vec![position("Tech Liaison", 1, "support")],
            18 => vec![position("Media Liaison", 1, "support")],
            _ => seats(),
        };
        zones.push(json!({"name":format!("Section {n}"),"kind":"section","mapRef":format!("S{n}"),"positions":ps}));
    }
    for n in 1..=9 {
        let ps = if [2, 5, 8].contains(&n) {
            vec![position("Door Usher", 1, "door")]
        } else {
            vec![
                position("Door Usher", 2, "door"),
                position("Greeter", 1, "greeter"),
            ]
        };
        zones.push(json!({"name":format!("Door D{n}"),"kind":"door","mapRef":format!("D{n}"),"positions":ps}));
    }
    for n in 1..=5 {
        let ps = if n < 3 {
            vec![position("Door Usher", 2, "door")]
        } else {
            vec![
                position("Greeter", 4, "greeter"),
                position("Door Usher", 3, "door"),
                position("Crowd Control", 3, "crowd"),
            ]
        };
        zones.push(json!({"name":format!("Door X{n}"),"kind":"door","mapRef":format!("X{n}"),"positions":ps}));
    }
    for n in 1..=3 {
        zones.push(json!({"name":format!("Door M{n}"),"kind":"door","mapRef":format!("M{n}"),"positions":[position("Door Usher",2,"door")]}));
    }
    for (name, map, ps) in [
        (
            "Altar & Pulpit",
            Some("altar"),
            vec![position("Altar Protocol", 12, "protocol")],
        ),
        (
            "Worship Team",
            Some("worship"),
            vec![position("Stage Support", 6, "support")],
        ),
        (
            "Apostle's Chairs",
            Some("apostle"),
            vec![position("Chair Protocol", 9, "protocol")],
        ),
        (
            "Protocol Row",
            Some("protocol"),
            vec![position("Protocol Support", 6, "protocol")],
        ),
        (
            "Technical Space",
            Some("tech"),
            vec![position("Tech Liaison", 3, "support")],
        ),
        (
            "Covered Walkway",
            Some("walkway"),
            vec![position("Walkway Control", 4, "crowd")],
        ),
        (
            "Main Gate",
            None,
            vec![
                position("Greeter", 20, "greeter"),
                position("Crowd Control", 12, "crowd"),
            ],
        ),
        (
            "Registration Area",
            None,
            vec![position("Registration Desk", 24, "support")],
        ),
        (
            "Car Park",
            None,
            vec![position("Traffic Direction", 30, "crowd")],
        ),
        (
            "Rotunda",
            None,
            vec![position("Ministers' Escort", 12, "protocol")],
        ),
        (
            "Overflow",
            None,
            vec![position("Overflow Seating", 30, "seat")],
        ),
        (
            "First Aid Point",
            None,
            vec![position("First Aid Support", 8, "support")],
        ),
    ] {
        zones.push(json!({"name":name,"kind":"area","mapRef":map,"positions":ps}));
    }
    json!({"zones":zones})
}
pub async fn add_position(c: &mut AsyncPgConnection, zone: &Value, mut p: Value) -> Result<Value> {
    required(&p, "name")?;
    one_of(
        &p,
        "duty",
        &[
            "seat", "crowd", "greeter", "door", "lead", "protocol", "support",
        ],
    )?;
    one_of(&p, "spot", &["FL", "FR", "BL", "BR"])?;
    if !p["spot"].is_null() && !spot_section(zone) {
        return Err(ApiError::new(
            422,
            "INVALID_SPOT",
            "This section cannot have usher spots",
        ));
    }
    if !p["required"].is_null() && p["required"].as_u64().is_none() {
        return Err(ApiError::bad("required must be a non-negative integer"));
    }
    if !p["isLeadRole"].is_null() && !p["isLeadRole"].is_boolean() {
        return Err(ApiError::bad("isLeadRole must be a boolean"));
    }
    if !p["spot"].is_null() && !p["required"].is_null() && p["required"] != 1 {
        return Err(ApiError::new(
            422,
            "SPOT_CAPACITY",
            "Spot headcount must be one",
        ));
    }
    let positions = db::children(c, Entity::Position, s(zone, "id")).await?;
    p["id"] = json!(id());
    p["zoneId"] = zone["id"].clone();
    p["order"] = json!(positions.len());
    p["required"] = json!(if p["spot"].is_null() {
        p["required"].as_u64().unwrap_or(1)
    } else {
        1
    });
    p["isLeadRole"] = json!(p["isLeadRole"].as_bool().unwrap_or(false));
    if p["duty"].is_null() {
        p["duty"] = json!(if !p["spot"].is_null() {
            "seat"
        } else if p["isLeadRole"] == true {
            "lead"
        } else {
            "support"
        });
    }
    db::save(c, Entity::Position, &p).await?;
    Ok(p)
}
pub async fn add_zone(
    c: &mut AsyncPgConnection,
    event: &str,
    mut z: Value,
    auto_seats: bool,
) -> Result<Value> {
    db::get(c, Entity::Event, event).await?;
    required(&z, "name")?;
    one_of(&z, "kind", &["aisle", "section", "door", "area"])?;
    required(&z, "kind")?;
    let n = db::children(c, Entity::Zone, event).await?.len();
    z["id"] = json!(id());
    z["eventId"] = json!(event);
    z["order"] = json!(n);
    if z["mapRef"].is_null() {
        z["mapRef"] = json!(map_ref(s(&z, "name")));
    }
    let ps = z
        .as_object_mut()
        .and_then(|o| o.remove("positions"))
        .and_then(|p| p.as_array().cloned())
        .unwrap_or_else(|| {
            if auto_seats && spot_section(&z) {
                seats()
            } else {
                vec![]
            }
        });
    db::save(c, Entity::Zone, &z).await?;
    for p in ps {
        add_position(c, &z, p).await?;
    }
    Ok(z)
}
pub async fn structure(c: &mut AsyncPgConnection, event: &str, source: Option<&str>) -> Result<()> {
    let zones = if let Some(source) = source {
        db::get(c, Entity::Event, source).await?;
        let mut zs = db::children(c, Entity::Zone, source).await?;
        zs.sort_by_key(|z| z["order"].as_u64().unwrap_or(0));
        for z in &mut zs {
            let mut ps = db::children(c, Entity::Position, s(z, "id")).await?;
            ps.sort_by_key(|p| p["order"].as_u64().unwrap_or(0));
            z["positions"] = json!(ps);
        }
        zs
    } else {
        template()["zones"].as_array().cloned().unwrap_or_default()
    };
    for z in zones {
        add_zone(c, event, z, false).await?;
    }
    Ok(())
}
pub async fn create(c: &mut AsyncPgConnection, mut v: Value, actor: &str) -> Result<Value> {
    validate(&v)?;
    let settings = db::get(c, Entity::Settings, "settings").await?;
    let source = v["copyStructureFrom"]
        .as_str()
        .or(settings["currentEventId"].as_str())
        .map(str::to_owned);
    let use_template = v["useTemplate"] == true;
    v.as_object_mut().unwrap().remove("copyStructureFrom");
    v.as_object_mut().unwrap().remove("useTemplate");
    stamp(&mut v);
    if v["status"].is_null() {
        v["status"] = json!("draft")
    }
    v["rosterStatus"] = json!("draft");
    v["publishedAt"] = Value::Null;
    db::save(c, Entity::Event, &v).await?;
    structure(
        c,
        s(&v, "id"),
        if use_template {
            None
        } else {
            source.as_deref()
        },
    )
    .await?;
    activity(c, actor, "event", format!("created {}", s(&v, "name"))).await?;
    Ok(v)
}
pub async fn apply_template(c: &mut AsyncPgConnection, event: &str) -> Result<()> {
    db::get(c, Entity::Event, event).await?;
    let existing = db::children(c, Entity::Zone, event).await?;
    for z in template()["zones"].as_array().unwrap() {
        if let Some(found) = existing.iter().find(|e| {
            e["name"] == z["name"] || (!z["mapRef"].is_null() && e["mapRef"] == z["mapRef"])
        }) {
            let ps = db::children(c, Entity::Position, s(found, "id")).await?;
            for p in z["positions"].as_array().unwrap() {
                if !ps.iter().any(|x| {
                    x["name"] == p["name"] || (!p["spot"].is_null() && x["spot"] == p["spot"])
                }) {
                    add_position(c, found, p.clone()).await?;
                }
            }
        } else {
            add_zone(c, event, z.clone(), false).await?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn venue() {
        let t = template();
        let zs = t["zones"].as_array().unwrap();
        assert_eq!(zs.len(), 121);
        assert_eq!(zs.iter().filter(|z| z["kind"] == "section").count(), 83);
        assert_eq!(
            zs.iter()
                .flat_map(|z| z["positions"].as_array().unwrap())
                .filter(|p| !p["spot"].is_null())
                .count(),
            312
        );
    }
}
