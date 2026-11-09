use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Login {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember: bool,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Refresh {
    pub refresh_token: String,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Phone {
    pub phone: String,
    pub event_id: Option<String>,
    pub reason: Option<String>,
    pub code: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberInput {
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email: Option<String>,
    pub member_id: Option<String>,
    pub gender: Option<String>,
    pub unit: Option<String>,
    pub status: Option<String>,
    pub joined_year: Option<u16>,
}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventInput {
    pub name: String,
    pub code: String,
    pub series: Option<String>,
    pub day: Option<u16>,
    pub session: Option<String>,
    pub theme: Option<String>,
    pub description: Option<String>,
    pub date: String,
    pub venue: String,
    pub reporting_time: String,
    pub briefing_time: Option<String>,
    pub start_time: Option<String>,
    pub dress_code: Option<String>,
    pub status: Option<String>,
    pub instructions: Option<String>,
    pub copy_structure_from: Option<String>,
    pub use_template: Option<bool>,
}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentInput {
    pub member_id: String,
    pub zone_id: String,
    pub position_id: String,
    pub is_team_lead: Option<bool>,
    pub replace: Option<bool>,
}
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub series: Option<String>,
    pub unit: Option<String>,
    pub gender: Option<String>,
    pub role: Option<String>,
    pub event_id: Option<String>,
    pub assignment: Option<String>,
    pub zone_id: Option<String>,
    pub position_id: Option<String>,
    pub duty: Option<String>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub limit: Option<usize>,
    pub phone: Option<String>,
    pub include: Option<String>,
    pub format: Option<String>,
    pub covered: Option<bool>,
    #[serde(skip)]
    pub zone_names: Vec<String>,
}
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: AdminUser,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub fields: Option<std::collections::BTreeMap<String, String>>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZoneInput {
    pub name: String,
    pub kind: String,
    pub map_ref: Option<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZonePatch {
    pub name: Option<String>,
    pub map_ref: Option<String>,
    pub order: Option<u32>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PositionInput {
    pub name: String,
    pub required: Option<u32>,
    pub is_lead_role: Option<bool>,
    pub duty: Option<String>,
    pub spot: Option<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PositionPatch {
    pub name: Option<String>,
    pub required: Option<u32>,
    pub is_lead_role: Option<bool>,
    pub duty: Option<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberPatch {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub member_id: Option<String>,
    pub gender: Option<String>,
    pub unit: Option<String>,
    pub status: Option<String>,
    pub joined_year: Option<u16>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventPatch {
    pub name: Option<String>,
    pub code: Option<String>,
    pub series: Option<String>,
    pub day: Option<u16>,
    pub session: Option<String>,
    pub theme: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
    pub venue: Option<String>,
    pub reporting_time: Option<String>,
    pub briefing_time: Option<String>,
    pub start_time: Option<String>,
    pub dress_code: Option<String>,
    pub status: Option<String>,
    pub instructions: Option<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentPatch {
    pub zone_id: Option<String>,
    pub position_id: Option<String>,
    pub is_team_lead: Option<bool>,
    pub status: Option<String>,
    pub unavailable_reason: Option<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamRole {
    pub role: String,
    pub leader_id: Option<String>,
    pub coordinates: Option<Vec<String>>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsPatch {
    pub current_event_id: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub show_teammate_phones: Option<bool>,
    pub allow_self_confirm: Option<bool>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberIds {
    pub member_ids: Vec<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BulkAssign {
    pub member_ids: Vec<String>,
    pub zone_id: String,
    pub position_id: String,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyAssignments {
    pub to_event_id: String,
    pub member_ids: Option<Vec<String>>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyRoster {
    pub from_event_id: String,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurrentEvent {
    pub event_id: String,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZoneOrder {
    pub zone_ids: Vec<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PositionOrder {
    pub position_ids: Vec<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyTemplate {
    pub mode: String,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportCommit {
    pub update_existing: bool,
    pub assign: bool,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ForgotPassword {
    pub email: String,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResetPassword {
    pub token: String,
    pub password: String,
}
/// Validate transport shapes without discarding explicit nulls in PATCH requests.
pub fn validate(op: &str, body: &serde_json::Value) -> crate::errors::api::Result<()> {
    use crate::errors::api::ApiError;
    use serde::de::DeserializeOwned;
    fn check<T: DeserializeOwned>(body: &serde_json::Value) -> crate::errors::api::Result<()> {
        serde_json::from_value::<T>(body.clone())
            .map(|_| ())
            .map_err(|e| {
                let mut error = ApiError::bad("Invalid request fields");
                error.fields = Some(serde_json::json!({"body":e.to_string()}));
                error
            })
    }
    if body.as_object().is_some_and(|object| {
        object
            .values()
            .any(|v| v.as_str().is_some_and(|s| s.len() > 65536))
    }) {
        return Err(ApiError::bad("A field exceeds the maximum length"));
    }
    if !body.is_object() {
        return Err(ApiError::bad("Request must be a JSON object"));
    }
    match op {
        "auth_login" => check::<Login>(body),
        "auth_refresh" => check::<Refresh>(body),
        "auth_forgot" => check::<ForgotPassword>(body),
        "auth_reset" => check::<ResetPassword>(body),
        "event_create" => check::<EventInput>(body),
        "event_patch" => check::<EventPatch>(body),
        "member_create" => check::<MemberInput>(body),
        "member_patch" => check::<MemberPatch>(body),
        "member_role" => check::<TeamRole>(body),
        "zone_create" => check::<ZoneInput>(body),
        "zone_patch" => check::<ZonePatch>(body),
        "position_create" => check::<PositionInput>(body),
        "position_patch" => check::<PositionPatch>(body),
        "zone_order" => check::<ZoneOrder>(body),
        "position_order" => check::<PositionOrder>(body),
        "zone_template" => check::<ApplyTemplate>(body),
        "assignment_create" => check::<AssignmentInput>(body),
        "assignment_patch" => check::<AssignmentPatch>(body),
        "assignment_bulk" => check::<BulkAssign>(body),
        "assignment_bulk_remove" | "member_bulk_delete" => check::<MemberIds>(body),
        "assignment_copy" => check::<CopyAssignments>(body),
        "roster_copy" => check::<CopyRoster>(body),
        "settings_patch" => check::<SettingsPatch>(body),
        "settings_current" => check::<CurrentEvent>(body),
        "imports_commit" => check::<ImportCommit>(body),
        "public_lookup" | "public_confirm" | "public_unavailable" | "public_available"
        | "team_auth_code" | "team_auth_verify" => check::<Phone>(body),
        _ => Ok(()),
    }?;
    let non_nullable: &[&str] = match op {
        "member_patch" => &["firstName", "lastName", "phone", "status"],
        "event_patch" => &["name", "code", "date", "venue", "reportingTime", "status"],
        "zone_patch" => &["name", "order"],
        "position_patch" => &["name", "required", "isLeadRole", "duty"],
        "assignment_patch" => &["zoneId", "positionId", "isTeamLead", "status"],
        "settings_patch" => &[
            "contactName",
            "contactPhone",
            "showTeammatePhones",
            "allowSelfConfirm",
        ],
        _ => &[],
    };
    for key in non_nullable {
        if body.get(*key).is_some_and(serde_json::Value::is_null) {
            return Err(ApiError::bad(format!("{key} cannot be null")));
        }
    }
    Ok(())
}
