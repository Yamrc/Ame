use crate::entity::player::PlayerEntity;

pub fn clear(player: &mut PlayerEntity) {
    player.clear();
}

pub fn remove_by_id(player: &mut PlayerEntity, id: i64) {
    let Some(index) = player.queue.iter().position(|x| x.id == id) else {
        return;
    };
    player.queue.remove(index);

    match player.current_index {
        Some(_) if player.queue.is_empty() => player.current_index = None,
        Some(current) if current > index => player.current_index = Some(current - 1),
        Some(current) if current == index => {
            if current >= player.queue.len() {
                player.current_index = player.queue.len().checked_sub(1);
            }
        }
        _ => {}
    }
}
