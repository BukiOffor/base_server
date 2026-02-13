use super::*;

#[derive(Queryable, Identifiable, Debug, Clone, Selectable)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[diesel(column_name = password_hash)]
    pub password: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_seen: Option<NaiveDateTime>,
}



