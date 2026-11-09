use super::*;
use diesel_async::RunQueryDsl;
use queries::Roster;
pub async fn periodic_notifications(c: &mut AsyncPgConnection) -> Result<()> {
    let today = chrono::Utc::now().date_naive();
    for ev in db::list(c, Entity::Event).await? {
        if matches!(s(&ev, "status"), "completed" | "archived") {
            continue;
        }
        if let Ok(date) = chrono::NaiveDate::parse_from_str(s(&ev, "date"), "%Y-%m-%d")
            && (0..=21).contains(&(date - today).num_days())
        {
            let v = json!({"id":format!("upcoming:{}",s(&ev,"id")),"title":"Event is approaching","body":format!("{} is {} days away",s(&ev,"name"),(date-today).num_days()),"at":now(),"tone":"info","read":false});
            diesel::sql_query(
                "INSERT INTO roster_notifications(id,data) VALUES($1,$2) ON CONFLICT DO NOTHING",
            )
            .bind::<diesel::sql_types::Text, _>(s(&v, "id"))
            .bind::<diesel::sql_types::Jsonb, _>(&v)
            .execute(c)
            .await?;
        }
    }
    let settings = db::get(c, Entity::Settings, "settings").await?;
    if let Some(event) = settings["currentEventId"].as_str() {
        let r = Roster::load(c, event).await?;
        let n = r.stats()["unassigned"].as_u64().unwrap_or(0);
        if n > 0 {
            let v = json!({"id":format!("unassigned:{event}:{today}"),"title":"Unassigned members remaining","body":format!("{n} members have no position"),"at":now(),"tone":"warn","read":false});
            diesel::sql_query(
                "INSERT INTO roster_notifications(id,data) VALUES($1,$2) ON CONFLICT DO NOTHING",
            )
            .bind::<diesel::sql_types::Text, _>(s(&v, "id"))
            .bind::<diesel::sql_types::Jsonb, _>(&v)
            .execute(c)
            .await?;
        }
    }
    Ok(())
}
