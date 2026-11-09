use super::*;
use calamine::{Reader, open_workbook_auto_from_rs};
use diesel::{
    sql_query,
    sql_types::{Jsonb, Text},
};
use diesel_async::RunQueryDsl;
use std::{collections::HashSet, io::Cursor};
fn header(h: &str) -> Option<&'static str> {
    let h: String = h
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect();
    Some(match h.as_str() {
        "firstname" | "first" | "givenname" => "firstName",
        "lastname" | "last" | "surname" => "lastName",
        "name" | "fullname" => "fullName",
        "phone" | "phonenumber" | "mobile" | "tel" | "telephone" => "phone",
        "email" | "emailaddress" => "email",
        "gender" | "sex" => "gender",
        "unit" | "team" | "teamunit" => "unit",
        "zone" | "aisle" | "zoneaisle" | "area" => "zone",
        "position" | "role" => "position",
        _ => return None,
    })
}
pub async fn preview(
    c: &mut AsyncPgConnection,
    file: &str,
    bytes: &[u8],
    event: &str,
) -> Result<Value> {
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(ApiError::bad("File exceeds 5 MB"));
    }
    db::get(c, Entity::Event, event).await?;
    let ext = file.rsplit('.').next().unwrap_or("").to_lowercase();
    let mut data: Vec<Vec<String>> = vec![];
    if ext == "csv" {
        let mut r = csv::ReaderBuilder::new().flexible(true).from_reader(bytes);
        data.push(
            r.headers()
                .map_err(|_| ApiError::bad("Invalid CSV headers"))?
                .iter()
                .map(str::to_owned)
                .collect(),
        );
        for row in r.records() {
            data.push(
                row.map_err(|_| ApiError::bad("Invalid CSV row"))?
                    .iter()
                    .map(str::to_owned)
                    .collect(),
            );
            if data.len() > 10001 {
                return Err(ApiError::bad("At most 10,000 rows are allowed"));
            }
        }
    } else if matches!(ext.as_str(), "xlsx" | "xls") {
        let mut book = open_workbook_auto_from_rs(Cursor::new(bytes))
            .map_err(|_| ApiError::bad("Invalid Excel workbook"))?;
        let range = book
            .worksheet_range_at(0)
            .ok_or_else(|| ApiError::bad("Workbook has no worksheets"))?
            .map_err(|_| ApiError::bad("Cannot read worksheet"))?;
        if range.height() > 10001 || range.width() > 100 {
            return Err(ApiError::bad("Workbook exceeds 10,000 rows or 100 columns"));
        }
        data = range
            .rows()
            .map(|r| r.iter().map(ToString::to_string).collect())
            .collect();
    } else {
        return Err(ApiError::bad("Upload a .csv, .xlsx or .xls file"));
    }
    let headers = data
        .as_slice()
        .first()
        .ok_or_else(|| ApiError::bad("File is empty"))?;
    let mapping: Vec<_> = headers.iter().map(|h| header(h)).collect();
    let columns: serde_json::Map<String, Value> = headers
        .iter()
        .zip(&mapping)
        .filter_map(|(h, m)| m.map(|m| (h.clone(), json!(m))))
        .collect();
    let members = db::list(c, Entity::Member).await?;
    let zones = db::children(c, Entity::Zone, event).await?;
    let r = queries::Roster::load(c, event).await?;
    let mut seen = HashSet::new();
    let mut rows = vec![];
    let (mut new, mut update, mut errors) = (0, 0, 0);
    for (i, row) in data.iter().enumerate().skip(1) {
        if row.iter().all(|s| s.trim().is_empty()) {
            continue;
        }
        let mut v = json!({});
        for (j, value) in row.iter().enumerate() {
            if let Some(Some(field)) = mapping.get(j) {
                v[*field] = json!(value.trim());
            }
        }
        if s(&v, "firstName").is_empty() && s(&v, "lastName").is_empty() {
            let full = s(&v, "fullName").to_owned();
            let mut parts = full.split_whitespace();
            v["firstName"] = json!(parts.next().unwrap_or(""));
            v["lastName"] = json!(parts.collect::<Vec<_>>().join(" "));
        }
        let mut errs = vec![];
        let mut warnings = vec![];
        if s(&v, "firstName").is_empty() || s(&v, "lastName").is_empty() {
            errs.push("First and last name are required".to_owned());
        }
        match normalize_phone(s(&v, "phone")) {
            Ok(phone) => {
                if !seen.insert(phone.clone()) {
                    errs.push("Phone repeated within file".into());
                }
                v["phone"] = json!(phone)
            }
            Err(_) => errs.push("Invalid or missing phone".into()),
        };
        if !s(&v, "email").is_empty() && !s(&v, "email").contains('@') {
            errs.push("Invalid email".into())
        }
        if !s(&v, "gender").is_empty() && !matches!(s(&v, "gender"), "Male" | "Female") {
            errs.push("Gender must be Male or Female".into())
        }
        let existing = members.iter().find(|m| m["phone"] == v["phone"]);
        let zone = zones
            .iter()
            .find(|z| s(z, "name").eq_ignore_ascii_case(s(&v, "zone")));
        if !s(&v, "zone").is_empty() {
            if let Some(z) = zone {
                let p = r.positions.iter().find(|p| {
                    p["zoneId"] == z["id"] && s(p, "name").eq_ignore_ascii_case(s(&v, "position"))
                });
                if let Some(p) = p {
                    if !p["spot"].is_null()
                        && r.assignments
                            .iter()
                            .any(|a| a["positionId"] == p["id"] && active(a))
                    {
                        warnings.push("Seat is already taken".to_owned());
                    }
                } else {
                    warnings.push(format!("Position not found: {}", s(&v, "position")));
                }
            } else {
                warnings.push(format!("Zone not found: {}", s(&v, "zone")));
            }
        }
        let status = if !errs.is_empty() {
            errors += 1;
            "error"
        } else if existing.is_some() {
            update += 1;
            "update"
        } else {
            new += 1;
            "new"
        };
        rows.push(json!({"row":i+1,"data":v,"status":status,"errors":errs,"warnings":warnings,"existingMemberId":existing.map(|m|m["id"].clone())}));
    }
    let preview = json!({"importId":id(),"fileName":file,"eventId":event,"columns":columns,"rows":rows,"summary":{"total":new+update+errors,"new":new,"update":update,"error":errors}});
    sql_query(
        "INSERT INTO roster_imports(id,data,expires_at) VALUES($1,$2,now()+interval '1 hour')",
    )
    .bind::<Text, _>(s(&preview, "importId"))
    .bind::<Jsonb, _>(&preview)
    .execute(c)
    .await?;
    if errors > 0 {
        notify(
            c,
            "Import validation issues",
            "Some rows have invalid or duplicate phone numbers",
            "warn",
        )
        .await?;
    }
    Ok(preview)
}
pub async fn commit(c: &mut AsyncPgConnection, id: &str, p: &Value, actor: &str) -> Result<Value> {
    let preview =
        sql_query("DELETE FROM roster_imports WHERE id=$1 AND expires_at>now() RETURNING data")
            .bind::<Text, _>(id)
            .get_result::<db::Document>(c)
            .await?
            .data;
    let event = s(&preview, "eventId");
    let zones = db::children(c, Entity::Zone, event).await?;
    let (mut added, mut updated, mut skipped, mut assigned) = (0, 0, 0, 0);
    for row in preview["rows"]
        .as_array()
        .ok_or_else(|| ApiError::bad("Invalid preview"))?
    {
        if row["status"] == "error" || (row["status"] == "update" && p["updateExisting"] != true) {
            skipped += 1;
            continue;
        }
        let mut v = json!({});
        for key in members::FIELDS {
            let val = &row["data"][*key];
            if !val.is_null() && val != "" {
                v[*key] = val.clone();
            }
        }
        let current = db::member_by_phone(c, s(&v, "phone")).await?;
        let m = if let Some(m) = current {
            if p["updateExisting"] != true {
                skipped += 1;
                continue;
            }
            updated += 1;
            members::update_inner(c, s(&m, "id"), &v, actor, false).await?
        } else {
            added += 1;
            members::create_inner(c, v, actor, false).await?
        };
        if p["assign"] == true
            && let Some(z) = zones
                .iter()
                .find(|z| s(z, "name").eq_ignore_ascii_case(s(&row["data"], "zone")))
        {
            let ps = db::children(c, Entity::Position, s(z, "id")).await?;
            if let Some(pos) = ps
                .iter()
                .find(|pos| s(pos, "name").eq_ignore_ascii_case(s(&row["data"], "position")))
            {
                let occupied = !pos["spot"].is_null()
                    && db::children(c, Entity::Assignment, event)
                        .await?
                        .iter()
                        .any(|a| {
                            a["positionId"] == pos["id"] && active(a) && a["memberId"] != m["id"]
                        });
                if !occupied {
                    assignments::assign(c,event,&json!({"memberId":m["id"],"zoneId":z["id"],"positionId":pos["id"],"replace":false}),actor,false).await?;
                    assigned += 1;
                }
            }
        }
    }
    let counts = json!({"added":added,"updated":updated,"skipped":skipped,"assigned":assigned});
    activity(
        c,
        actor,
        "import",
        format!("imported {added} new, {updated} updated, {skipped} skipped"),
    )
    .await?;
    notify(c, "Import completed", &counts.to_string(), "success").await?;
    Ok(counts)
}
