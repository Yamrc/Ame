mod key;
mod lifecycle;
mod render;
mod slot;

use std::collections::HashMap;
use std::time::Duration;

use nekowg::{Context, Pixels, ScrollHandle, px};

use crate::app::runtime::AppRuntime;

use self::key::PageKey;
use self::slot::{FrozenPage, PageInstance};

const FROZEN_TTL: Duration = Duration::from_secs(300);

pub struct PageHostView {
    runtime: AppRuntime,
    page_scroll_handle: ScrollHandle,
    active: Option<PageInstance>,
    frozen: HashMap<PageKey, FrozenPage>,
    pending_scroll_restore: Option<Pixels>,
}

impl PageHostView {
    pub fn new(
        runtime: AppRuntime,
        page_scroll_handle: ScrollHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let host = Self {
            runtime,
            page_scroll_handle,
            active: None,
            frozen: HashMap::new(),
            pending_scroll_restore: Some(px(0.)),
        };
        host.spawn_prune_task(cx);
        host
    }
}
