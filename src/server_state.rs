use std::collections::HashSet;

use uuid::Uuid;

#[derive(Default)]
pub struct ServerState {
    pub available_players: HashSet<Uuid>,
}

impl ServerState {
    pub fn add_available_player(&mut self, player_id: &Uuid) {
        self.available_players.insert(*player_id);
    }
    pub fn remove_available_player(&mut self, player_id: &Uuid) {
        self.available_players.remove(player_id);
    }
}
