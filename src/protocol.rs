use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientRequestError {
    CannotCreateMatch,
    Match404,
    PermissionDenied,
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum ServerMessage {
    AskPassword,
    WrongPassword,
    /***
      ID has been assigned to a new connected client
    */
    AssignId(Uuid),
    BadRequest(ClientRequestError),
    /***
      Response to `GetOpponents`
    */
    ListOpponents(Vec<Uuid>),
    /***
      Response for Challenger that the Match(Uuid) has been started
    */
    MatchAccepted(Uuid),
    /***
      Response for Guesser that the Match(Uuid) has been started
    */
    MatchStarted(Uuid),
    /***
      Status message for Challenger about progress of the match
      (match_id, attempts, hints, latest_attempt)
    */
    MatchAttempt(Uuid, u32, u32, String),
    IncorrectGuess(Uuid, u32),
    /***
      Challenger can send a hint to Guesser
      (match_id, hint)
    */
    MatchHint(Uuid, String),
    /***
      Match can end by either giving up or guessing the correct word
      (match_id, attempts, hints, solved)
    */
    MatchEnded(Uuid, u32, u32, bool),
    Disconnect,
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum ClientMessage {
    AnswerPassword(String),
    GetOpponents,
    RequestMatch(Uuid, String),
    GuessAttempt(Uuid, String),
    SendHint(Uuid, String),
    GiveUp(Uuid),
    LeaveGame,
}
