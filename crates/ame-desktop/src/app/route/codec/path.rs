pub(super) fn normalize_path(path: &str) -> String {
    let mut value = path.trim().to_string();
    if value.is_empty() {
        value = "/".to_string();
    }
    if !value.starts_with('/') {
        value.insert(0, '/');
    }
    if value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    value
}
