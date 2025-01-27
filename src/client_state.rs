use indoc::{indoc, printdoc};
use log::error;
use uuid::Uuid;

use crate::{
    protocol::{ClientMessage, ServerMessage},
    validation::is_valid_word,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Initial,
    WaitingForPassword,
    WaitingForPasswordValidation,
    MainMenu,
    ChoosingOpponent(Vec<Uuid>),
    ChallengePlayer(Uuid),
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
            ServerMessage::ListOpponents(opponents) => {
                if opponents.is_empty() {
                    printdoc! {"
                        No available opponents to match with.
                        Please wait for other players to connect

                        please specify what action you would like to take by typing a number:

                        (0) Quit
                        (1) List and challenge available opponents
                    "}
                } else {
                    self.status = State::ChoosingOpponent(opponents.clone());
                    let text_block = opponents
                        .iter()
                        .enumerate()
                        .map(|(idx, opp)| format!("({}) - {}", idx + 1, opp))
                        .collect::<Vec<String>>()
                        .join("\n");
                    printdoc! {"

                        Available opponents: 

                        {text_block}

                        (0) Go back
                        
                    "};
                }
            }
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

    pub fn update_from_user(&mut self, input: &str) -> Option<ClientMessage> {
        let status = &self.status.clone();
        match status {
            State::WaitingForPassword => {
                // TODO maybe we can skip 2 steps and send message directly
                self.status = State::SendPassword(input.to_string());
                None
            }
            State::MainMenu => match input {
                "0" => {
                    self.status = State::Quit(
                        indoc! {"
                        Thank you for tryin out this game.
                        See you next time!
                    "}
                        .to_string(),
                    );
                    Some(ClientMessage::LeaveGame)
                }
                "1" => {
                    printdoc! {"
                        Getting list of available opponents...

                    "};
                    Some(ClientMessage::GetOpponents)
                }
                _ => {
                    printdoc! {
                        "Invalid input"
                    };
                    None
                }
            },
            State::ChoosingOpponent(opponents) => {
                let challenged_player = input
                    .parse::<usize>()
                    .ok()
                    .and_then(|input_idx| opponents.get(input_idx - 1));
                if let Some(challenged_player) = challenged_player {
                    self.status = State::ChallengePlayer(*challenged_player);
                    printdoc! {"
                        Specify word to guess:

                    "};
                    None
                } else {
                    let text_block = opponents
                        .iter()
                        .enumerate()
                        .map(|(idx, opp)| format!("({}) - {}", idx + 1, opp))
                        .collect::<Vec<String>>()
                        .join("\n");
                    printdoc! {"
                    Invalid input.

                    Please specify correct number next to the opponent you want to challenge

                    Available opponents: 

                    {text_block}

                    (0) Go back
                        
                    "};
                    None
                }
            }

            State::ChallengePlayer(opponent) => {
                if is_valid_word(input) {
                    Some(ClientMessage::RequestMatch((*opponent, input.to_string())))
                } else {
                    printdoc! {"
                        Please specify a single word with only alphabetic lowercase characters.

                    "};
                    None
                }
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
                );
                None
            }
        }
    }

    pub fn set_state(&mut self, status: State) {
        self.status = status;
    }

    pub fn process(&mut self) -> Option<ClientMessage> {
        let status = &self.status.clone();
        match status {
            State::Initial
            | State::WaitingForPasswordValidation
            | State::ChoosingOpponent(_)
            | State::ChallengePlayer(_) => {
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

                    (0) Quit
                    (1) List and challenge available opponents
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
