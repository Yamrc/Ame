use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use futures::FutureExt;
use gpui::{
    App, AppContext, Asset, AssetLogger, Entity, ImageAssetLoader, ImageCache, ImageCacheError,
    ImageCacheItem, RenderImage, Resource, Window, hash,
};

pub struct LruImageCache {
    items: HashMap<u64, ImageCacheItem>,
    lru: VecDeque<u64>,
    max_items: usize,
}

impl LruImageCache {
    pub fn new(cx: &mut App, max_items: usize) -> Entity<Self> {
        let max_items = max_items.max(1);
        let entity = cx.new(move |_| Self {
            items: HashMap::new(),
            lru: VecDeque::new(),
            max_items,
        });
        cx.observe_release(&entity, |this: &mut LruImageCache, cx| {
            this.drop_all(cx);
        })
        .detach();
        entity
    }

    pub fn default_for_app(cx: &mut App) -> Entity<Self> {
        Self::new(cx, 24)
    }

    fn touch(&mut self, key: u64) {
        if let Some(index) = self.lru.iter().position(|existing| *existing == key) {
            self.lru.remove(index);
        }
        self.lru.push_back(key);
    }

    fn evict_to_budget(&mut self, window: &mut Window, cx: &mut App) {
        while self.items.len() > self.max_items {
            let Some(key) = self.lru.pop_front() else {
                break;
            };
            if let Some(mut item) = self.items.remove(&key)
                && let Some(Ok(image)) = item.get()
            {
                cx.drop_image(image, Some(window));
            }
        }
    }

    fn drop_all(&mut self, cx: &mut App) {
        for (_, mut item) in std::mem::take(&mut self.items) {
            if let Some(Ok(image)) = item.get() {
                cx.drop_image(image, None);
            }
        }
        self.lru.clear();
    }
}

impl ImageCache for LruImageCache {
    fn load(
        &mut self,
        resource: &Resource,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        let key = hash(resource);
        if self.items.contains_key(&key) {
            self.touch(key);
            return self.items.get_mut(&key).and_then(ImageCacheItem::get);
        }

        let fut = AssetLogger::<ImageAssetLoader>::load(resource.clone(), cx);
        let task = cx.background_executor().spawn(fut).shared();

        self.items.insert(key, ImageCacheItem::Loading(task.clone()));
        self.touch(key);
        self.evict_to_budget(window, cx);

        let view = window.current_view();
        window
            .spawn(cx, async move |cx| {
                _ = task.await;
                cx.on_next_frame(move |_, cx| {
                    cx.notify(view);
                });
            })
            .detach();

        None
    }
}
