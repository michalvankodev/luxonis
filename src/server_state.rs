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

    pub fn create_new_match(&self, opponent: (&Uuid, Uuid), guess_word: String) -> Option<Uuid> {
        // TODO finish this function
        if !self.available_players.contains(opponent) {
            None
        }
    }
}
