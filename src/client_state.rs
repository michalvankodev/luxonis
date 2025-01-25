use indoc::printdoc;
use log::error;
use uuid::Uuid;

use crate::protocol::ServerMessage;

#[derive(Debug)]
pub enum State {
    Initial,
    WaitingForPassword,
    WaitingForPasswordValidation,
    Quit,
    SendPassword(String),
}

#[derive(Debug)]
pub struct ClientState {
    pub player_id: Option<Uuid>,
    pub status: State,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            player_id: None,
            status: State::Initial,
        }
    }
}

impl ClientState {
    pub fn update_from_server(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::AskPassword => {
                self.status = State::WaitingForPassword;
            }
            ServerMessage::WrongPassword => todo!(),
            ServerMessage::AssignId(_) => todo!(),
            ServerMessage::BadRequest => todo!(),
            ServerMessage::ListOpponents(_) => todo!(),
            ServerMessage::MatchAccepted(_) => todo!(),
            ServerMessage::MatchDeclined(_) => todo!(),
            ServerMessage::MatchStatus(_) => todo!(),
            ServerMessage::MatchHint(_) => todo!(),
            ServerMessage::MatchEnded(_) => todo!(),
            ServerMessage::Disconnect => {
                self.status = State::Quit;
            }
        }
    }

    pub fn update_from_user(&mut self, input: &str) {
        match self.status {
            State::WaitingForPassword => {
                self.status = State::SendPassword(input.to_string());
            }
            State::Quit => {
                printdoc!(
                    r#"
                        See you next time!
                    "#
                );
            }
            _ => {
                error!(
                    "User shouldn't be able to input anything while in {:?} state",
                    self.status
                )
            }
        }
    }

    pub fn set_state(&mut self, status: State) {
        self.status = status;
    }
}
