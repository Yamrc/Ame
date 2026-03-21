use nekowg::AppContext;

use crate::app::runtime::{AppRuntime, KEY_WINDOW_CLOSE_BEHAVIOR};
use crate::domain::session::push_shell_error;
use crate::domain::settings::CloseBehavior;

pub fn set_close_behavior<C: AppContext>(runtime: &AppRuntime, value: CloseBehavior, cx: &mut C) {
    let changed = runtime.shell.update(cx, |shell, cx| {
        if shell.close_behavior == value {
            return false;
        }
        shell.close_behavior = value;
        cx.notify();
        true
    });

    if !changed {
        return;
    }

    if let Some(settings) = runtime.services.settings_store.as_ref()
        && let Err(err) = settings.set(KEY_WINDOW_CLOSE_BEHAVIOR, &value)
    {
        push_shell_error(runtime, format!("Failed to save close behavior: {err}"), cx);
    }
}
