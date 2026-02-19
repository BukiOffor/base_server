use super::*;

#[derive(Selectable, Serialize, Deserialize, Queryable, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::projects)]
pub struct ProjectDto {
    pub id: uuid::Uuid,
    pub organisation_id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_public: bool,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProject {
    pub id: Uuid,
    pub organisation_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub is_public: bool,
}

impl NewProject {
    pub fn build(mut self) -> Self {
        self.id = Uuid::now_v7();
        self.created_at = Some(chrono::Utc::now().naive_utc());
        self.updated_at = Some(chrono::Utc::now().naive_utc());
        self
    }
}
