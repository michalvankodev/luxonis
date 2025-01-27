use indoc::printdoc;
use log::error;
use uuid::Uuid;

use crate::protocol::{ClientMessage, ServerMessage};

#[derive(Debug, Clone)]
pub enum State {
    Initial,
    WaitingForPassword,
    WaitingForPasswordValidation,
    MainMenu,
    /***
        Quit the application with goodbye msg
    */
    Quit(String),
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
            ServerMessage::WrongPassword => {
                self.status = State::Quit("Wrong password. Please try again!".to_string());
            }
            ServerMessage::AssignId(id) => {
                self.player_id = Some(id);
                self.status = State::MainMenu;
            }
            ServerMessage::BadRequest => todo!(),
            ServerMessage::ListOpponents(_) => todo!(),
            ServerMessage::MatchAccepted(_) => todo!(),
            ServerMessage::MatchDeclined(_) => todo!(),
            ServerMessage::MatchStatus(_) => todo!(),
            ServerMessage::MatchHint(_) => todo!(),
            ServerMessage::MatchEnded(_) => todo!(),
            ServerMessage::Disconnect => {
                self.status =
                    State::Quit("Server has unexpectedly ended the connection".to_string());
            }
        }
    }

    pub fn update_from_user(&mut self, input: &str) {
        match self.status {
            State::WaitingForPassword => {
                self.status = State::SendPassword(input.to_string());
            }
            // State::Quit(_) => {
            //     printdoc!(
            //         r#"
            //             See you next time!
            //         "#
            //     );
            // }
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

    pub fn process(&mut self) -> Option<ClientMessage> {
        let status = &self.status.clone();
        match status {
            State::Initial | State::WaitingForPasswordValidation => {
                // let server_msg = wait_for_server_msg(&mut connection).await?;
                // client_state.update_from_server(server_msg);
                None
            }
            State::WaitingForPassword => {
                printdoc!(
                    r#"
                        Welcome to WordGuesser.
                        Please authenticate yourself with a _not really secret_ **password**.
                    "#
                );
                // let input = wait_for_user_input().await;
                // client_state.update_from_user(&input);
                None
            }
            State::SendPassword(password) => {
                printdoc! {
                    "Attempting to authenticate with provided password"
                };
                self.status = State::WaitingForPasswordValidation;
                Some(ClientMessage::AnswerPassword(password.to_string()))
            }
            State::MainMenu => {
                printdoc! {
                    "Please specify what action you would like to take by typing a number:

                    (1) List available opponents
                    (2) Challenge opponent
                    (4) Quit
                    "
                };
                None
            }
            State::Quit(reason) => {
                printdoc!(
                    r#"
                        {reason}
                        See you next time!
                    "#
                );
                Some(ClientMessage::LeaveGame)
            } // _ => {}
        }
    }
}
