use nekowg::Context;

use crate::app::audio_bridge::AudioBridgeEntity;
use crate::app::runtime::AppRuntime;
use crate::domain::session as auth;

pub(super) fn with_audio_bridge<R>(
    runtime: &AppRuntime,
    f: impl FnOnce(&mut AudioBridgeEntity) -> R,
) -> Result<R, String> {
    let Some(bridge) = runtime.services.audio_bridge.as_ref() else {
        return Err("Audio bridge is not initialized".to_string());
    };
    let mut guard = bridge
        .lock()
        .map_err(|err| format!("Failed to lock audio bridge: {err}"))?;
    Ok(f(&mut guard))
}

pub(super) fn with_audio_bridge_or_error<T, R>(
    runtime: &AppRuntime,
    cx: &mut Context<T>,
    context: &str,
    f: impl FnOnce(&mut AudioBridgeEntity) -> R,
) -> Option<R> {
    match with_audio_bridge(runtime, f) {
        Ok(value) => Some(value),
        Err(err) => {
            auth::push_shell_error(runtime, format!("{context}: {err}"), cx);
            None
        }
    }
}

pub(super) fn set_shell_error_if_changed<T>(
    runtime: &AppRuntime,
    message: String,
    cx: &mut Context<T>,
) {
    let same_error = runtime.shell.read(cx).error.as_deref() == Some(message.as_str());
    if !same_error {
        auth::set_shell_error(runtime, Some(message), cx);
    }
}
