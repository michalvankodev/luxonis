use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum ServerMessage {
    AskPassword = 0,
    WrongPassword = 1,
    /***
      ID has been assigned to a new connected client
    */
    AssignId(Uuid) = 2,
    BadRequest = 3,
    /***
      Response to `GetOpponents`
    */
    ListOpponents(Vec<Uuid>),
    /***
      Response for Challenger that the Match with (Guesser) has been started
    */
    MatchAccepted(Uuid),
    /***
      Response for Challenger that the (Guesser) has been declined the match
    */
    MatchDeclined(Uuid),
    /***
      Status message for Challenger about progress of the match
      (Guesser, attempts)
    */
    MatchStatus((Uuid, u32)),
    /***
      Challenger can send a hint to Guesser
      (Challenger, Hint)
    */
    MatchHint((Uuid, String)),
    /***
      Match can end by either giving up or guessing the correct word
      (Challenger, Guesser, CorrectAnswer, Attempts)
    */
    MatchEnded((Uuid, Uuid, bool, u32)),
    Disconnect,
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum ClientMessage {
    AnswerPassword(String),
    GetOpponents,
    RequestMatch((Uuid, String)),
    AcceptMatch(Uuid),
    DeclineMatch(Uuid),
    GuessAttempt((Uuid, String)),
    SendHint((Uuid, String)),
    GiveUp(Uuid),
    LeaveGame,
}
