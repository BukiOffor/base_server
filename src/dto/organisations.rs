use super::*;

#[derive(Selectable, Serialize, Deserialize, Queryable, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::organisations)]
pub struct OrganisationDto {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::organisations)]
pub struct NewOrganisation {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl NewOrganisation {
    pub fn build(mut self) -> Self {
        self.id = Uuid::now_v7();
        self.created_at = Some(chrono::Utc::now().naive_utc());
        self.updated_at = Some(chrono::Utc::now().naive_utc());
        self
    }
}
