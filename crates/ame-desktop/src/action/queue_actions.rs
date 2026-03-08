use crate::entity::player::PlayerEntity;

pub fn clear(player: &mut PlayerEntity) {
    player.clear();
}

pub fn index_of(player: &PlayerEntity, id: i64) -> Option<usize> {
    player.index_of_id(id)
}

pub fn remove_by_id(player: &mut PlayerEntity, id: i64) {
    if let Some(index) = index_of(player, id) {
        player.remove_at(index);
    }
}
