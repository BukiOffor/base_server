use super::*;

#[derive(Selectable, Serialize, Deserialize, Queryable, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::users)]
pub struct UserDto {
    pub id: uuid::Uuid,
    pub username: String,
    #[diesel(column_name = password_hash)]
    #[serde(skip_serializing)]
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub last_seen: Option<NaiveDateTime>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub id: Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[diesel(column_name = password_hash)]
    pub password: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub last_seen: Option<NaiveDateTime>,
}
impl NewUser {
    pub fn build(mut self) -> Result<Self, ModuleError> {
        let hashed = helpers::password_hasher(&self.password)?;
        self.id = Uuid::now_v7();
        self.password = hashed;
        self.created_at = Some(chrono::Utc::now().naive_utc());
        self.updated_at = Some(chrono::Utc::now().naive_utc());
        self.last_seen = Some(chrono::Utc::now().naive_utc());
        Ok(self.clone())
    }
}
