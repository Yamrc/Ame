use nekowg::Context;

use crate::domain::player;

use super::NextPageView;

impl NextPageView {
    pub(super) fn play_item(&mut self, item_id: i64, cx: &mut Context<Self>) {
        player::play_queue_item(&self.runtime, item_id, cx);
    }

    pub(super) fn remove_item(&mut self, item_id: i64, cx: &mut Context<Self>) {
        player::remove_queue_item(&self.runtime, item_id, cx);
    }

    pub(super) fn clear_queue(&mut self, cx: &mut Context<Self>) {
        player::clear_queue(&self.runtime, cx);
    }
}
