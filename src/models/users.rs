use std::io::Write;

use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    serialize::{self, Output, ToSql},
    sql_types::Text,
};

use super::*;

#[derive(Queryable, Identifiable, Debug, Clone, Selectable, Deserialize, Serialize)]
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
    pub organisation_id: Option<Uuid>,
    pub role: RoleType,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    AsExpression,
    FromSqlRow,
    PartialEq,
    Eq,
    Hash,
    utoipa::ToSchema,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum RoleType {
    Admin,
    Manager,
    Root,
    User,
}

impl FromSql<Text, diesel::pg::Pg> for RoleType {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        match std::str::from_utf8(bytes.as_bytes())? {
            "Admin" => Ok(RoleType::Admin),
            "Manager" => Ok(RoleType::Manager),
            "Root" => Ok(RoleType::Root),
            "User" => Ok(RoleType::User),
            s => Err(format!("Unrecognized role type: {}", s).into()),
        }
    }
}

impl ToSql<Text, diesel::pg::Pg> for RoleType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let s = match self {
            RoleType::Admin => "Admin",
            RoleType::Manager => "Manager",
            RoleType::Root => "Root",
            RoleType::User => "User",
        };
        out.write(s.as_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
