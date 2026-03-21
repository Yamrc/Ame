use nekowg::AppContext;

use crate::app::runtime::{AppRuntime, KEY_HOME_ARTIST_LANGUAGE};
use crate::domain::session::push_shell_error;
use crate::domain::settings::HomeArtistLanguage;

pub fn set_home_artist_language<C: AppContext>(
    runtime: &AppRuntime,
    value: HomeArtistLanguage,
    cx: &mut C,
) {
    let changed = runtime.app.update(cx, |app, cx| {
        if app.home_artist_language == value {
            return false;
        }
        app.set_home_artist_language(value);
        cx.notify();
        true
    });

    if !changed {
        return;
    }

    if let Some(settings) = runtime.services.settings_store.as_ref()
        && let Err(err) = settings.set(KEY_HOME_ARTIST_LANGUAGE, &value)
    {
        push_shell_error(
            runtime,
            format!("Failed to save home artist language: {err}"),
            cx,
        );
    }
}
