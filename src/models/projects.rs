use super::*;

#[derive(Queryable, Identifiable, Debug, Clone, Selectable, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::projects)]
pub struct Project {
    pub id: Uuid,
    pub organisation_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_public: bool,
}
