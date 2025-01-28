use std::collections::{HashMap, HashSet};

use uuid::Uuid;

#[derive(Default)]
pub enum MatchState {
    #[default]
    Active,
    GivenUp,
    Solved,
}

#[derive(Default)]
pub struct Match {
    pub id: Uuid,
    pub challenger: Uuid,
    pub guesser: Uuid,
    pub attempts: u32,
    pub hints: Vec<String>,
    pub guess_word: String,
    pub state: MatchState,
}

impl Match {
    pub fn new((challenger, guesser): (&Uuid, &Uuid), guess_word: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            challenger: *challenger,
            guesser: *guesser,
            attempts: 0,
            hints: Vec::<String>::new(),
            guess_word: guess_word.to_string(),
            state: MatchState::Active,
        }
    }

    pub fn attempt(&mut self, guess: &str) {
        self.attempts += 1;

        if guess.eq(&self.guess_word) {
            self.state = MatchState::Solved;
        }
    }

    pub fn add_hint(&mut self, hint: &str) {
        self.hints.push(hint.to_string());
    }
}

#[derive(Default)]
pub struct ServerState {
    pub available_players: HashSet<Uuid>,
    pub active_matches: HashMap<Uuid, Match>,
    pub finished_matches: HashMap<Uuid, Match>,
}

impl ServerState {
    pub fn add_available_player(&mut self, player_id: &Uuid) {
        self.available_players.insert(*player_id);
    }
    pub fn remove_available_player(&mut self, player_id: &Uuid) {
        self.available_players.remove(player_id);
    }

    pub fn create_new_match(
        &mut self,
        player_duo: (&Uuid, &Uuid),
        guess_word: &str,
    ) -> Option<Uuid> {
        if !self.available_players.contains(player_duo.1)
            || !self.available_players.contains(player_duo.0)
        {
            return None;
        }
        let new_match = Match::new(player_duo, guess_word);
        let id = new_match.id;
        self.active_matches.insert(new_match.id, new_match);
        self.available_players.remove(player_duo.0);
        self.available_players.remove(player_duo.1);

        Some(id)
    }
}
