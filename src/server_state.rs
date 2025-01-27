use std::collections::HashSet;

use uuid::Uuid;

#[derive(Default)]
pub struct ServerState {
    pub available_opponents: HashSet<Uuid>,
}

impl ServerState {}
