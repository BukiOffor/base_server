use super::*;
pub const FIELDS: &[&str] = &[
    "firstName",
    "lastName",
    "phone",
    "email",
    "memberId",
    "gender",
    "unit",
    "status",
    "joinedYear",
];
pub async fn validate(c: &mut AsyncPgConnection, v: &mut Value) -> Result<()> {
    for key in ["firstName", "lastName", "email", "memberId", "unit"] {
        if let Some(value) = v[key].as_str() {
            v[key] = json!(value.trim());
        }
    }
    if v["email"] == "" {
        v["email"] = Value::Null;
    }
    required(v, "firstName")?;
    required(v, "lastName")?;
    v["phone"] = json!(normalize_phone(required(v, "phone")?)?);
    one_of(v, "gender", &["Male", "Female"])?;
    one_of(v, "status", &["active", "inactive"])?;
    if let Some(y) = v["joinedYear"].as_u64()
        && !(1900..=chrono::Utc::now()
            .format("%Y")
            .to_string()
            .parse()
            .unwrap_or(2100))
            .contains(&y)
    {
        return Err(ApiError::bad("Invalid joinedYear"));
    }
    if !v["email"].is_null() && !s(v, "email").contains('@') {
        return Err(ApiError::bad("Invalid email"));
    }
    if s(v, "memberId").is_empty() {
        v["memberId"] = Value::Null;
    }
    for other in db::member_conflicts(c, v).await? {
        if other["id"] == v["id"] {
            continue;
        }
        if other["phone"] == v["phone"] {
            let mut e = ApiError::new(
                409,
                "DUPLICATE_PHONE",
                format!(
                    "Phone already belongs to {} {} ({})",
                    s(&other, "firstName"),
                    s(&other, "lastName"),
                    s(&other, "id")
                ),
            );
            e.details = Some(
                json!({"member":{"id":other["id"],"name":format!("{} {}",s(&other,"firstName"),s(&other,"lastName"))}}),
            );
            return Err(e);
        }
        if !v["memberId"].is_null() && other["memberId"] == v["memberId"] {
            return Err(ApiError::new(
                409,
                "DUPLICATE_MEMBER_ID",
                "Member ID is already used",
            ));
        }
    }
    Ok(())
}
pub async fn create(c: &mut AsyncPgConnection, v: Value, actor: &str) -> Result<Value> {
    create_inner(c, v, actor, true).await
}
pub async fn create_inner(
    c: &mut AsyncPgConnection,
    mut v: Value,
    actor: &str,
    audit: bool,
) -> Result<Value> {
    stamp(&mut v);
    v["status"] = json!(v["status"].as_str().unwrap_or("active"));
    v["role"] = json!("usher");
    v["coordinates"] = json!([]);
    validate(c, &mut v).await?;
    db::save(c, Entity::Member, &v).await?;
    if audit {
        activity(
            c,
            actor,
            "member",
            format!("added {} {}", s(&v, "firstName"), s(&v, "lastName")),
        )
        .await?;
    }
    Ok(v)
}
pub async fn update(c: &mut AsyncPgConnection, id: &str, p: &Value, actor: &str) -> Result<Value> {
    update_inner(c, id, p, actor, true).await
}
pub async fn update_inner(
    c: &mut AsyncPgConnection,
    id: &str,
    p: &Value,
    actor: &str,
    audit: bool,
) -> Result<Value> {
    let mut v = db::get(c, Entity::Member, id).await?;
    merge(&mut v, p, FIELDS)?;
    validate(c, &mut v).await?;
    db::save(c, Entity::Member, &v).await?;
    if audit {
        activity(c, actor, "member", "updated member").await?;
    }
    Ok(v)
}
pub async fn role(c: &mut AsyncPgConnection, id: &str, p: &Value, actor: &str) -> Result<Value> {
    one_of(p, "role", &["usher", "coordinator", "leader"])?;
    let role = required(p, "role")?;
    let mut m = db::get(c, Entity::Member, id).await?;
    if role == "coordinator" {
        if let Some(leader) = p["leaderId"].as_str() {
            let l = db::get(c, Entity::Member, leader).await.map_err(|error| {
                if error.status == axum::http::StatusCode::NOT_FOUND {
                    ApiError::new(422, "INVALID_LEADER", "leaderId must refer to a leader")
                } else {
                    error
                }
            })?;
            if s(&l, "role") != "leader" || leader == id {
                return Err(ApiError::new(
                    422,
                    "INVALID_LEADER",
                    "leaderId must refer to a leader",
                ));
            }
        }
        if !p["coordinates"].is_null()
            && p["coordinates"].as_array().is_none_or(|a| {
                a.iter()
                    .any(|x| x.as_str().is_none_or(|s| s.trim().is_empty()))
            })
        {
            return Err(ApiError::bad("coordinates must be zone names"));
        }
    }
    if s(&m, "role") == "leader" && role != "leader" {
        for mut co in db::list(c, Entity::Member).await? {
            if s(&co, "leaderId") == id {
                co["leaderId"] = Value::Null;
                co["updatedAt"] = json!(now());
                db::save(c, Entity::Member, &co).await?;
            }
        }
    }
    m["role"] = json!(role);
    m["leaderId"] = if role == "coordinator" {
        p["leaderId"].clone()
    } else {
        Value::Null
    };
    m["coordinates"] = if role == "coordinator" {
        p["coordinates"]
            .as_array()
            .cloned()
            .map(Value::Array)
            .unwrap_or(json!([]))
    } else {
        json!([])
    };
    m["updatedAt"] = json!(now());
    db::save(c, Entity::Member, &m).await?;
    activity(c, actor, "team", format!("set member role to {role}")).await?;
    Ok(m)
}
