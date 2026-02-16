use super::*;

#[derive(Queryable, Identifiable, Debug, Clone, Selectable, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::organisations)]
pub struct Organisation {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
