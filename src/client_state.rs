use indoc::{indoc, printdoc};
use log::error;
use uuid::Uuid;

use crate::{
    protocol::{ClientMessage, ClientRequestError, ServerMessage},
    validation::is_valid_word,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Initial,
    WaitingForPassword,
    SendPassword(String),
    WaitingForPasswordValidation,
    MainMenu,
    ChoosingOpponent(Vec<Uuid>),
    ChallengePlayer(Uuid),
    InGameChallenger(Uuid),
    InGameGuesser(Uuid),
    /// Quit the application with goodbye msg
    Disconnect(String),
    Quit,
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
    /// Process message from server
    pub fn update_from_server(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::AskPassword => {
                self.status = State::WaitingForPassword;
            }
            ServerMessage::WrongPassword => {
                self.status = State::Disconnect("Wrong password. Please try again!".to_string());
            }
            ServerMessage::AssignId(id) => {
                self.player_id = Some(id);
                self.status = State::MainMenu;
            }
            ServerMessage::BadRequest(client_err) => match client_err {
                ClientRequestError::CannotCreateMatch => {
                    printdoc! {"
                        Cannot create a match with selected opponent. They are no longer available.

                    "}

                    self.status = State::MainMenu;
                }
                ClientRequestError::Match404 => {
                    printdoc! {"
                        Unexpected error occured. Match doesn't exist anymore. 

                    "}

                    self.status = State::MainMenu;
                }
                ClientRequestError::PermissionDenied => {
                    printdoc! {"
                        You cannot perform this action.

                    "}
                }
            },
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
            ServerMessage::MatchAccepted(id) => {
                printdoc! {"
                    Match between you and your opponent has started.

                    If you see your opponent struggling you can provide a hint for them:
                        
                    "};
                self.status = State::InGameChallenger(id);
            }
            ServerMessage::MatchStarted(id) => {
                printdoc! {"
                        You have been challenged to a game.

                        Start guessing!
                        
                "};

                self.status = State::InGameGuesser(id);
            }
            ServerMessage::MatchAttempt(_id, attempts, hints, latest_attempt) => {
                printdoc! {"
                    Opponent has guessed {latest_attempt}.
                    They've made {attempts} attempts so far and you've given them {hints} hints.

                "}
            }
            ServerMessage::IncorrectGuess(_id, attempts) => {
                printdoc! {"
                    Incorrect. So far, you've made {attempts} attempts.
                    Try again!

                "}
            }
            ServerMessage::MatchHint(_id, hint) => {
                printdoc! {"
                    Challenger provides a hint:
                    {hint}

                "}
            }
            ServerMessage::MatchEnded(_id, attempts, hints, is_solved) => {
                if matches!(self.status, State::InGameChallenger(_)) {
                    let solved_msg = if is_solved {
                        "Your opponent has guessed the right word!"
                    } else {
                        "Your opponent has given up"
                    };
                    printdoc! {"
                        {solved_msg}
                        They took {attempts} attempts. You've given them {hints} hints.

                    "}
                } else {
                    let solved_msg = if is_solved {
                        "Congratulations!!! You have guessed the correct word!"
                    } else {
                        // FIXME match can be cancelled by challenger disconnecting
                        "It's OK to admit defeat, better luck next time"
                    };
                    printdoc! {"
                       {solved_msg}
                           
                       "}
                }
                self.status = State::MainMenu;
            }
            ServerMessage::Disconnect => {
                self.status = State::Quit;
            }
        }
    }

    /// Update the state and optionally send a new message to the server if appropriate
    pub fn update_from_user(&mut self, input: &str) -> Option<ClientMessage> {
        let status = &self.status.clone();
        match status {
            State::WaitingForPassword => {
                self.status = State::SendPassword(input.to_string());
                None
            }
            State::MainMenu => match input {
                "0" => {
                    self.status = State::Disconnect(
                        indoc! {"
                        Thank you for trying out this game.
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
                if input.eq("0") {
                    self.status = State::MainMenu;
                    return None;
                }
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
                    Some(ClientMessage::RequestMatch(*opponent, input.to_string()))
                } else {
                    printdoc! {"
                        Please specify a single word with only alphabetic lowercase characters.

                    "};
                    None
                }
            }
            State::InGameChallenger(match_id) => {
                Some(ClientMessage::SendHint(*match_id, input.to_string()))
            }
            State::InGameGuesser(match_id) => {
                if input.eq("give up") {
                    return Some(ClientMessage::GiveUp(*match_id));
                }
                Some(ClientMessage::GuessAttempt(*match_id, input.to_string()))
            }
            _ => {
                error!(
                    "User shouldn't be able to input anything while in {:?} state",
                    self.status
                );
                None
            }
        }
    }

    /// Process state changes
    pub fn process(&mut self) -> Option<ClientMessage> {
        let status = &self.status.clone();
        match status {
            State::Initial
            | State::WaitingForPasswordValidation
            | State::ChoosingOpponent(_)
            | State::ChallengePlayer(_)
            | State::InGameChallenger(_)
            | State::InGameGuesser(_)
            | State::Quit => None,

            State::WaitingForPassword => {
                printdoc! {"

                        Welcome to WordGuesser.
                        Please authenticate yourself with a _not really secret_ **password**.
                    
                "};
                None
            }
            State::SendPassword(password) => {
                printdoc! {"
                    Attempting to authenticate with provided password

                "};
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
            State::Disconnect(reason) => {
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
