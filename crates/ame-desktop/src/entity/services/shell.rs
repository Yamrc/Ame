use nekowg::AppContext;

use crate::entity::app::CloseBehavior;
use crate::entity::runtime::{AppRuntime, KEY_WINDOW_CLOSE_BEHAVIOR};

use super::auth::push_shell_error;

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
        push_shell_error(runtime, format!("保存关闭行为失败: {err}"), cx);
    }
}
