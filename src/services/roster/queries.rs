use super::*;
use diesel_async::RunQueryDsl;
use std::collections::HashMap;
pub struct Roster {
    pub event: Value,
    pub zones: Vec<Value>,
    pub positions: Vec<Value>,
    pub assignments: Vec<Value>,
    pub members: HashMap<String, Value>,
}
impl Roster {
    pub async fn load(c: &mut AsyncPgConnection, event: &str) -> Result<Self> {
        let event = db::get(c, Entity::Event, event).await?;
        let mut zones = db::children(c, Entity::Zone, s(&event, "id")).await?;
        zones.sort_by_key(|v| v["order"].as_u64().unwrap_or(0));
        let positions=diesel::sql_query("SELECT p.data FROM roster_positions p JOIN roster_zones z ON z.id=p.zone_id WHERE z.event_id=$1 ORDER BY (p.data->>'order')::int").bind::<diesel::sql_types::Text,_>(s(&event,"id")).load::<db::Document>(c).await.map_err(ApiError::from)?.into_iter().map(|r|r.data).collect();
        let assignments = db::children(c, Entity::Assignment, s(&event, "id")).await?;
        let members = db::list(c, Entity::Member)
            .await?
            .into_iter()
            .map(|m| (s(&m, "id").to_owned(), m))
            .collect();
        Ok(Self {
            event,
            zones,
            positions,
            assignments,
            members,
        })
    }
    pub fn member(&self, a: &Value) -> Value {
        self.members
            .get(s(a, "memberId"))
            .cloned()
            .unwrap_or(Value::Null)
    }
    pub fn position(&self, a: &Value) -> Value {
        self.positions
            .iter()
            .find(|p| p["id"] == a["positionId"])
            .cloned()
            .unwrap_or(Value::Null)
    }
    pub fn zone(&self, a: &Value) -> Value {
        self.zones
            .iter()
            .find(|z| z["id"] == a["zoneId"])
            .cloned()
            .unwrap_or(Value::Null)
    }
    pub fn covered(&self, a: &Value) -> bool {
        let p = self.position(a);
        let n = self
            .assignments
            .iter()
            .filter(|x| x["positionId"] == a["positionId"] && active(x))
            .count() as u64;
        n >= if !p["spot"].is_null() {
            1
        } else {
            p["required"].as_u64().unwrap_or(1)
        }
    }
    pub fn zone_stat(&self, z: &Value) -> Value {
        let assigned = self
            .assignments
            .iter()
            .filter(|a| a["zoneId"] == z["id"] && active(a))
            .count();
        let required: u64 = self
            .positions
            .iter()
            .filter(|p| p["zoneId"] == z["id"])
            .map(|p| p["required"].as_u64().unwrap_or(1))
            .sum();
        let coverage = if required == 0 {
            if assigned > 0 { 100.0 } else { 0.0 }
        } else {
            (assigned as f64 / required as f64 * 100.0)
                .round()
                .min(100.0)
        };
        let lead = self
            .assignments
            .iter()
            .find(|a| a["zoneId"] == z["id"] && active(a) && a["isTeamLead"] == true)
            .map(|a| brief(&self.member(a), true));
        json!({"zoneId":z["id"],"assigned":assigned,"required":required,"coverage":coverage,"lead":lead})
    }
    pub fn stats(&self) -> Value {
        let total = self
            .members
            .values()
            .filter(|m| m["status"] == "active")
            .count();
        let assigned = self.assignments.iter().filter(|a| active(a)).count();
        let confirmed = self
            .assignments
            .iter()
            .filter(|a| a["status"] == "confirmed")
            .count();
        let unavailable = self.assignments.iter().filter(|a| !active(a)).count();
        let needs = self
            .assignments
            .iter()
            .filter(|a| !active(a) && !self.covered(a))
            .count();
        let required: u64 = self
            .positions
            .iter()
            .map(|p| p["required"].as_u64().unwrap_or(1))
            .sum();
        json!({"totalMembers":total,"assigned":assigned,"unassigned":total.saturating_sub(assigned),"confirmed":confirmed,"positions":self.positions.len(),"zones":self.zones.len(),"required":required,"coverage":if total==0{0.0}else{(assigned as f64/total as f64*1000.0).round()/10.0},"unavailable":unavailable,"needsCover":needs})
    }
    pub fn enriched(&self, a: &Value) -> Value {
        let mut a = clean(a.clone());
        a["member"] = brief(&self.member(&a), true);
        a["zoneName"] = self.zone(&a)["name"].clone();
        let p = self.position(&a);
        a["positionName"] = p["name"].clone();
        a["duty"] = p["duty"].clone();
        a["spot"] = p["spot"].clone();
        a
    }
    pub fn absences(&self) -> Vec<Value> {
        let mut rows:Vec<_>=self.assignments.iter().filter(|a|!active(a)).map(|a|json!({"assignmentId":a["id"],"member":brief(&self.member(a),true),"zone":self.zone(a),"position":self.position(a),"reason":a["unavailableReason"],"at":a["unavailableAt"],"covered":self.covered(a)})).collect();
        sort(&mut rows, "at", true);
        rows
    }
    pub fn zones(&self) -> Vec<Value> {
        self.zones
            .iter()
            .map(|z| {
                let mut z = z.clone();
                z["stats"] = self.zone_stat(&z);
                z["positions"] = json!(
                    self.positions
                        .iter()
                        .filter(|p| p["zoneId"] == z["id"])
                        .collect::<Vec<_>>()
                );
                z
            })
            .collect()
    }
    pub fn map(&self, public: bool, scope: Option<&[String]>) -> Value {
        let published = self.event["rosterStatus"] == "published";
        let zones: Vec<_> = self
            .zones
            .iter()
            .map(|z| {
                let mut z = z.clone();
                if !public {
                    z["stats"] = self.zone_stat(&z);
                }
                z
            })
            .collect();
        let spots:Vec<_>=self.positions.iter().filter(|p|!p["spot"].is_null()).map(|p|{let occupant=self.assignments.iter().find(|a|a["positionId"]==p["id"]&&active(a));let away=if occupant.is_none(){self.assignments.iter().filter(|a|a["positionId"]==p["id"]&&!active(a)).max_by_key(|a|s(a,"unavailableAt"))}else{None};let z=self.zones.iter().find(|z|z["id"]==p["zoneId"]);let visible=!public&&scope.is_none_or(|scope|published&&z.is_some_and(|z|scope.iter().any(|n|n==s(z,"name"))));let mut v=json!({"positionId":p["id"],"zoneId":p["zoneId"],"spot":p["spot"],"name":p["name"]});if visible{v["occupant"]=occupant.map(|a|{let m=self.member(a);json!({"assignmentId":a["id"],"memberId":m["id"],"firstName":m["firstName"],"lastName":m["lastName"],"status":a["status"]})}).unwrap_or(Value::Null);v["away"]=away.map(|a|{let m=self.member(a);json!({"assignmentId":a["id"],"firstName":m["firstName"],"lastName":m["lastName"],"reason":a["unavailableReason"]})}).unwrap_or(Value::Null);}else{v["taken"]=json!(published&&occupant.is_some());v["needsCover"]=json!(published&&away.is_some());}v}).collect();
        json!({"zones":zones,"spots":spots})
    }
    pub fn by(&self, key: &str, values: &[&str]) -> Value {
        let mut out = json!({});
        for value in values {
            let ps: Vec<_> = self
                .positions
                .iter()
                .filter(|p| {
                    if key == "duty" {
                        s(p, key) == *value
                    } else {
                        self.zones
                            .iter()
                            .any(|z| z["id"] == p["zoneId"] && s(z, key) == *value)
                    }
                })
                .collect();
            let assigned = self
                .assignments
                .iter()
                .filter(|a| active(a) && ps.iter().any(|p| p["id"] == a["positionId"]))
                .count();
            let required: u64 = ps.iter().map(|p| p["required"].as_u64().unwrap_or(1)).sum();
            out[value] = json!({"assigned":assigned,"required":required});
        }
        out
    }
    pub fn view(&self, a: &Value, phones: bool, teammates: bool) -> Value {
        let mut v =
            json!({"assignment":clean(a.clone()),"zone":self.zone(a),"position":self.position(a)});
        let lead = self
            .assignments
            .iter()
            .find(|x| x["zoneId"] == a["zoneId"] && x["isTeamLead"] == true && active(x));
        v["teamLead"] = lead
            .map(|x| brief(&self.member(x), phones))
            .unwrap_or(Value::Null);
        let zone = self.zone(a);
        let mut cos: Vec<_> = self
            .members
            .values()
            .filter(|m| {
                s(m, "role") == "coordinator"
                    && m["coordinates"]
                        .as_array()
                        .is_some_and(|ns| ns.contains(&zone["name"]))
            })
            .collect();
        cos.sort_by_key(|m| format!("{} {}", s(m, "firstName"), s(m, "lastName")));
        v["coordinator"] = cos
            .as_slice()
            .first()
            .map(|m| brief(m, true))
            .unwrap_or(Value::Null);
        if teammates {
            let mut rows: Vec<_> = self
                .assignments
                .iter()
                .filter(|x| x["zoneId"] == a["zoneId"] && (active(x) || x["id"] == a["id"]))
                .collect();
            rows.sort_by_key(|x| {
                (
                    !x["isTeamLead"].as_bool().unwrap_or(false),
                    self.position(x)["order"].as_u64().unwrap_or(0),
                )
            });
            v["teammates"]=json!(rows.into_iter().map(|x|json!({"member":brief(&self.member(x),phones),"position":self.position(x),"isTeamLead":x["isTeamLead"],"isYou":x["memberId"]==a["memberId"]})).collect::<Vec<_>>());
        }
        v
    }
}
pub const DUTIES: &[&str] = &[
    "seat", "crowd", "greeter", "door", "lead", "protocol", "support",
];
pub async fn sessions(c: &mut AsyncPgConnection, event: &Value) -> Result<Vec<Value>> {
    let mut es = db::list(c, Entity::Event).await?;
    es.retain(|e| {
        if event["series"].is_null() || s(event, "series").is_empty() {
            e["id"] == event["id"]
        } else {
            e["series"] == event["series"]
        }
    });
    es.sort_by_key(|e| {
        (
            s(e, "date").to_owned(),
            if s(e, "session") == "morning" { 0 } else { 1 },
        )
    });
    Ok(es)
}
pub async fn lookup(c: &mut AsyncPgConnection, phone: &str, event: Option<&str>) -> Result<Value> {
    let phone = normalize_phone(phone)?;
    let settings = db::get(c, Entity::Settings, "settings").await?;
    let contact = json!({"name":settings["contactName"],"phone":settings["contactPhone"]});
    let m = db::member_by_phone(c, &phone).await?;
    let Some(m) = m else {
        return Ok(json!({"kind":"not-found","phone":phone,"contact":contact}));
    };
    let event = event.or(settings["currentEventId"].as_str());
    let Some(event) = event else {
        return Ok(
            json!({"kind":"member","member":brief(&m,false),"series":null,"sessions":[],"others":[],"contact":contact}),
        );
    };
    let current = db::get(c, Entity::Event, event).await?;
    let sessions = sessions(c, &current).await?;
    let mut result = vec![];
    for ev in &sessions {
        let mut entry = json!({"event":ev});
        if ev["rosterStatus"] == "published" {
            let r = Roster::load(c, s(ev, "id")).await?;
            if let Some(a) = r.assignments.iter().find(|a| a["memberId"] == m["id"]) {
                entry["view"] = r.view(a, settings["showTeammatePhones"] == true, true);
            }
        }
        result.push(entry);
    }
    let mut others = vec![];
    let mut all = db::list(c, Entity::Event).await?;
    sort(&mut all, "date", false);
    for ev in all.into_iter().filter(|e| {
        e["rosterStatus"] == "published"
            && e["status"] != "archived"
            && !sessions.iter().any(|x| x["id"] == e["id"])
    }) {
        let r = Roster::load(c, s(&ev, "id")).await?;
        if let Some(a) = r.assignments.iter().find(|a| a["memberId"] == m["id"]) {
            let mut view = r.view(a, settings["showTeammatePhones"] == true, false);
            view["event"] = ev;
            others.push(view);
        }
    }
    Ok(
        json!({"kind":"member","member":brief(&m,false),"series":current["series"],"sessions":result,"others":others,"contact":contact}),
    )
}
