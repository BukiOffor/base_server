use super::*;

#[derive(Queryable, Identifiable, Debug, Clone, Selectable, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::tours)]
pub struct Tour {
    pub id: Uuid,
    pub organisation_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub panorama_url: String,
    pub created_by: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Debug, Clone, Selectable, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::tour_links)]
pub struct TourLink {
    pub id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub yaw: f64,
    pub pitch: f64,
    pub label: Option<String>,
    pub created_at: NaiveDateTime,
}
