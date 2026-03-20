use ame_core::credential::AuthBundle;

const MUSIC_U: &str = "MUSIC_U";
const MUSIC_A: &str = "MUSIC_A";
const CSRF: &str = "__csrf";
const MUSIC_R_T: &str = "MUSIC_R_T";

pub fn build_cookie_header(bundle: &AuthBundle) -> Option<String> {
    let mut pairs = Vec::new();

    if let Some(music_u) = bundle.music_u.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{MUSIC_U}={music_u}"));
    }
    if let Some(music_a) = bundle.music_a.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{MUSIC_A}={music_a}"));
    }
    if let Some(csrf) = bundle.csrf.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{CSRF}={csrf}"));
    }
    if let Some(music_r_t) = bundle.music_r_t.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{MUSIC_R_T}={music_r_t}"));
    }

    if pairs.is_empty() {
        return None;
    }
    Some(pairs.join("; "))
}

pub fn merge_bundle_from_set_cookie(bundle: &mut AuthBundle, set_cookie: &[String]) -> bool {
    let mut changed = false;

    for raw in set_cookie {
        if let Some((key, value)) = parse_cookie_fragment(raw) {
            match key.as_str() {
                MUSIC_U => changed |= replace_if_changed(&mut bundle.music_u, value),
                MUSIC_A => changed |= replace_if_changed(&mut bundle.music_a, value),
                CSRF => changed |= replace_if_changed(&mut bundle.csrf, value),
                MUSIC_R_T => changed |= replace_if_changed(&mut bundle.music_r_t, value),
                _ => {}
            }
        }
    }

    changed
}

fn replace_if_changed(slot: &mut Option<String>, value: String) -> bool {
    if slot.as_ref() == Some(&value) {
        return false;
    }
    *slot = Some(value);
    true
}

fn parse_cookie_fragment(raw: &str) -> Option<(String, String)> {
    let first = raw.split(';').next()?.trim();
    let (key, value) = first.split_once('=')?;
    if key.trim().is_empty() || value.trim().is_empty() {
        return None;
    }
    Some((key.trim().to_string(), value.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use ame_core::credential::AuthBundle;

    use super::{build_cookie_header, merge_bundle_from_set_cookie};

    #[test]
    fn cookie_header_prioritizes_music_u_and_keeps_music_a() {
        let bundle = AuthBundle {
            music_u: Some("u".to_string()),
            music_a: Some("a".to_string()),
            csrf: Some("c".to_string()),
            music_r_t: None,
        };

        assert_eq!(
            build_cookie_header(&bundle).as_deref(),
            Some("MUSIC_U=u; MUSIC_A=a; __csrf=c")
        );
    }

    #[test]
    fn merge_only_updates_whitelisted_keys() {
        let mut bundle = AuthBundle::default();
        let changed = merge_bundle_from_set_cookie(
            &mut bundle,
            &[
                "MUSIC_A=guest; Path=/; HttpOnly".to_string(),
                "SID=ignored; Path=/".to_string(),
                "__csrf=token; Path=/".to_string(),
            ],
        );

        assert!(changed);
        assert_eq!(bundle.music_a.as_deref(), Some("guest"));
        assert_eq!(bundle.csrf.as_deref(), Some("token"));
        assert_eq!(bundle.music_u, None);
    }
}
