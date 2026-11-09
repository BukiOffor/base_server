//! OpenAPI is kept separately from HTTP handlers and domain services.
pub fn openapi() -> utoipa::openapi::OpenApi {
    serde_json::from_str(include_str!("roster.json")).expect("validated OpenAPI specification")
}
#[cfg(test)]
mod tests {
    #[test]
    fn valid_contract() {
        let api = super::openapi();
        assert!(api.paths.paths.len() > 50);
        assert!(
            api.components
                .unwrap()
                .security_schemes
                .contains_key("bearerAuth")
        );
    }

    #[test]
    fn every_required_endpoint_is_documented() {
        use std::collections::BTreeSet;
        fn normalize(path: &str) -> String {
            let mut out = String::new();
            let mut placeholder = false;
            for c in path.chars() {
                match c {
                    '{' => {
                        placeholder = true;
                        out.push_str("{}");
                    }
                    '}' => placeholder = false,
                    _ if !placeholder => out.push(c),
                    _ => {}
                }
            }
            out
        }
        let requirements = include_str!("../../API.md")
            .split("## 16. Endpoint index")
            .nth(1)
            .expect("Endpoint index");
        let expected: BTreeSet<_> = requirements
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let method = fields.next()?;
                let path = fields.next()?;
                if !["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&method)
                    || !path.starts_with('/')
                    || path == "/admin/reset-demo"
                {
                    return None;
                }
                Some(format!(
                    "{method} {}",
                    normalize(path.split('?').next().unwrap())
                ))
            })
            .collect();
        let spec: serde_json::Value = serde_json::from_str(include_str!("roster.json")).unwrap();
        let actual: BTreeSet<_> = spec["paths"]
            .as_object()
            .unwrap()
            .iter()
            .flat_map(|(path, ops)| {
                ops.as_object()
                    .unwrap()
                    .keys()
                    .map(move |method| format!("{} {}", method.to_uppercase(), normalize(path)))
            })
            .collect();
        assert_eq!(expected, actual);
    }
}
