fn main() {
    println!("cargo:rerun-if-env-changed=AME_LASTFM_API_KEY");
    println!("cargo:rerun-if-env-changed=AME_LASTFM_SHARED_SECRET");

    let has_api_key = std::env::var("AME_LASTFM_API_KEY")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    let has_shared_secret = std::env::var("AME_LASTFM_SHARED_SECRET")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());

    if !has_api_key || !has_shared_secret {
        println!(
            "cargo:warning=Last.fm credentials are not fully configured; Ame will build with Last.fm disabled at runtime"
        );
    }
}
