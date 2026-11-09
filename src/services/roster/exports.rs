use super::*;
use queries::Roster;
pub struct Download {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub filename: String,
}
fn cell(v: &Value) -> String {
    v.as_str().map(str::to_owned).unwrap_or_else(|| {
        if v.is_null() {
            String::new()
        } else {
            v.to_string()
        }
    })
}
fn safe(s: &str) -> String {
    if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.to_owned()
    }
}
pub fn file(rows: &[Vec<String>], format: &str, filename: String, title: &str) -> Result<Download> {
    let (bytes, content_type) = match format {
        "csv" => {
            let mut w = csv::Writer::from_writer(vec![]);
            for row in rows {
                w.write_record(row.iter().map(|s| safe(s)))
                    .map_err(|_| ApiError::new(500, "EXPORT_FAILED", "CSV generation failed"))?;
            }
            (
                w.into_inner()
                    .map_err(|_| ApiError::new(500, "EXPORT_FAILED", "CSV generation failed"))?,
                "text/csv; charset=utf-8",
            )
        }
        "xlsx" => {
            let mut book = rust_xlsxwriter::Workbook::new();
            let sheet = book.add_worksheet();
            for (i, row) in rows.iter().enumerate() {
                for (j, s) in row.iter().enumerate() {
                    sheet.write_string(i as u32, j as u16, s).map_err(|_| {
                        ApiError::new(500, "EXPORT_FAILED", "Excel generation failed")
                    })?;
                }
            }
            sheet.autofit();
            (
                book.save_to_buffer()
                    .map_err(|_| ApiError::new(500, "EXPORT_FAILED", "Excel generation failed"))?,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            )
        }
        "pdf" => (super::pdf::render(rows, title)?, "application/pdf"),
        _ => return Err(ApiError::bad("format must be csv, xlsx or pdf")),
    };
    Ok(Download {
        bytes,
        content_type,
        filename: format!("{filename}.{format}"),
    })
}
pub async fn export(
    c: &mut AsyncPgConnection,
    event: &str,
    kind: &str,
    format: &str,
    zone: Option<&str>,
) -> Result<Download> {
    let r = Roster::load(c, event).await?;
    let mut rows: Vec<Vec<String>> = vec![];
    match kind {
        "contacts" | "unassigned" => {
            let headers = if kind == "contacts" {
                vec!["Member", "Phone", "Email", "Unit", "Status"]
            } else {
                vec!["Member", "Phone", "Email", "Unit"]
            };
            rows.push(headers.iter().map(|s| s.to_string()).collect());
            let mut ms: Vec<_> = r
                .members
                .values()
                .filter(|m| {
                    kind == "contacts"
                        || (m["status"] == "active"
                            && !r
                                .assignments
                                .iter()
                                .any(|a| a["memberId"] == m["id"] && active(a)))
                })
                .collect();
            ms.sort_by_key(|m| format!("{} {}", s(m, "firstName"), s(m, "lastName")));
            for m in ms {
                let mut row = vec![
                    format!("{} {}", s(m, "firstName"), s(m, "lastName")),
                    cell(&m["phone"]),
                    cell(&m["email"]),
                    cell(&m["unit"]),
                ];
                if kind == "contacts" {
                    row.push(cell(&m["status"]));
                }
                rows.push(row);
            }
        }
        "absences" => {
            rows.push(
                ["Zone", "Position", "Member", "Phone", "Reason", "Covered"]
                    .map(str::to_owned)
                    .to_vec(),
            );
            for a in r.absences() {
                rows.push(vec![
                    cell(&a["zone"]["name"]),
                    cell(&a["position"]["name"]),
                    format!(
                        "{} {}",
                        s(&a["member"], "firstName"),
                        s(&a["member"], "lastName")
                    ),
                    cell(&a["member"]["phone"]),
                    cell(&a["reason"]),
                    cell(&a["covered"]),
                ]);
            }
        }
        "full" | "aisle" | "event" => {
            if kind == "aisle" {
                let z = zone.ok_or_else(|| ApiError::bad("zoneId is required for aisle export"))?;
                if !r.zones.iter().any(|zone| s(zone, "id") == z) {
                    return Err(ApiError::missing());
                }
            }
            let mut header = [
                "Zone",
                "Position",
                "Duty",
                "Spot",
                "Member",
                "Phone",
                "Team Lead",
                "Status",
            ]
            .map(str::to_owned)
            .to_vec();
            if kind == "event" {
                header.push("Unit".into());
            }
            rows.push(header);
            let mut assignments: Vec<_> = r
                .assignments
                .iter()
                .filter(|a| active(a) && zone.is_none_or(|z| s(a, "zoneId") == z))
                .collect();
            assignments.sort_by_key(|a| {
                (
                    r.zone(a)["order"].as_u64().unwrap_or(0),
                    r.position(a)["order"].as_u64().unwrap_or(0),
                )
            });
            for a in assignments {
                let m = r.member(a);
                let p = r.position(a);
                let mut row = vec![
                    cell(&r.zone(a)["name"]),
                    cell(&p["name"]),
                    cell(&p["duty"]),
                    cell(&p["spot"]),
                    format!("{} {}", s(&m, "firstName"), s(&m, "lastName")),
                    cell(&m["phone"]),
                    cell(&a["isTeamLead"]),
                    cell(&a["status"]),
                ];
                if kind == "event" {
                    row.push(cell(&m["unit"]));
                }
                rows.push(row);
            }
        }
        _ => return Err(ApiError::bad("Unknown export kind")),
    }
    let code: String = s(&r.event, "code")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let filename = format!(
        "{code}-D{}-{}-{kind}-roster",
        cell(&r.event["day"]),
        s(&r.event, "session")
    );
    let title = format!(
        "{} | {} | Day {} {} | Report {} | Start {}",
        s(&r.event, "name"),
        s(&r.event, "date"),
        cell(&r.event["day"]),
        s(&r.event, "session"),
        s(&r.event, "reportingTime"),
        s(&r.event, "startTime")
    );
    file(&rows, format, filename, &title)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn downloads() {
        let rows = vec![vec!["Name".into()], vec!["=1+1".into()]];
        let csv = file(&rows, "csv", "test".into(), "Test").unwrap();
        assert!(String::from_utf8(csv.bytes).unwrap().contains("'=1+1"));
        let x = file(&rows, "xlsx", "test".into(), "Test").unwrap();
        assert!(x.bytes.starts_with(b"PK"));
        let p = file(&rows, "pdf", "test".into(), "Test").unwrap();
        assert!(p.bytes.starts_with(b"%PDF-1.4"));
        assert!(p.bytes.ends_with(b"%%EOF"));
    }
}
