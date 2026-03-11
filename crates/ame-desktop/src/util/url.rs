pub fn image_resize_url(url: &str, new_value: &str) -> String {
    let key_eq = format!("{}=", "param");
    let (base_with_query, fragment) = match url.find('#') {
        Some(hash_pos) => (&url[..hash_pos], &url[hash_pos..]),
        None => (url, ""),
    };

    if let Some(query_start) = base_with_query.find('?') {
        let base = &base_with_query[..query_start];
        let mut query = &base_with_query[query_start + 1..];
        while query.starts_with('?') {
            query = &query[1..];
        }

        if let Some(param_start) = query.find(&key_eq) {
            let before_param = &query[..param_start];
            let after_key = &query[param_start + key_eq.len()..];
            let value_end = after_key.find('&').unwrap_or(after_key.len());
            let new_query = format!(
                "{}{}{}{}",
                before_param,
                key_eq,
                new_value,
                &after_key[value_end..]
            );
            format!("{}?{}{}", base, new_query, fragment)
        } else {
            let separator = if query.is_empty() { "" } else { "&" };
            format!(
                "{}?{}{}{}={}{}",
                base, query, separator, "param", new_value, fragment
            )
        }
    } else {
        format!("{}?{}={}{}", base_with_query, "param", new_value, fragment)
    }
}
