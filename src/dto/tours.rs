use super::*;

// ===================== Tour DTOs =====================

#[derive(Selectable, Serialize, Deserialize, Queryable, Debug, Clone, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::tours)]
pub struct TourDto {
    pub id: uuid::Uuid,
    pub organisation_id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub panorama_url: String,
    pub created_by: uuid::Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::tours)]
pub struct NewTour {
    pub id: Uuid,
    pub organisation_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub panorama_url: String,
    pub created_by: Uuid,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl NewTour {
    pub fn build(mut self, user_id: Uuid) -> Self {
        self.id = Uuid::now_v7();
        self.created_by = user_id;
        self.created_at = Some(chrono::Utc::now().naive_utc());
        self.updated_at = Some(chrono::Utc::now().naive_utc());
        self
    }
}

// ===================== TourLink DTOs =====================

#[derive(Selectable, Serialize, Deserialize, Queryable, Debug, Clone, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::tour_links)]
pub struct TourLinkDto {
    pub id: uuid::Uuid,
    pub source_node_id: uuid::Uuid,
    pub target_node_id: uuid::Uuid,
    pub yaw: f64,
    pub pitch: f64,
    pub label: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[diesel(table_name = crate::schema::tour_links)]
pub struct NewTourLink {
    pub id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub yaw: f64,
    pub pitch: f64,
    pub label: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

impl NewTourLink {
    pub fn build(mut self) -> Self {
        self.id = Uuid::now_v7();
        self.created_at = Some(chrono::Utc::now().naive_utc());
        self
    }
}

// ===================== Tour Graph (for viewer) =====================

#[derive(Serialize, Deserialize, Debug, Clone, utoipa::ToSchema)]
pub struct TourGraphNode {
    pub id: uuid::Uuid,
    pub name: String,
    pub panorama_url: String,
    pub links: Vec<TourGraphLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, utoipa::ToSchema)]
pub struct TourGraphLink {
    pub target: uuid::Uuid,
    pub yaw: f64,
    pub pitch: f64,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, utoipa::ToSchema)]
pub struct TourGraph {
    pub organisation_id: uuid::Uuid,
    pub rooms: Vec<TourGraphNode>,
}
