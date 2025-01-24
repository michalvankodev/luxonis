use uuid::Uuid;

pub enum ServerMessage {
    AskPassword,
    WrongPassword,
    /***
      ID has been assigned to a new connected client
    */
    AssignId(Uuid),
    BadRequest,
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
      (Challenger, Guesser, CorrectAnswer, Attempts)
    */
    MatchHint((Uuid, String)),
    /***
      Match can end by either giving up or guessing the correct word
      (Challenger, Guesser, CorrectAnswer, Attempts)
    */
    MatchEnded((Uuid, Uuid, bool, u32)),
}

pub enum ClientMessage {
    AnswerPassword(String),
    GetOpponents,
    RequestMatch((Uuid, String)),
    AcceptMatch(Uuid),
    DeclineMatch(Uuid),
    GuessAttempt((Uuid, String)),
    SendHint((Uuid, String)),
    GiveUp(Uuid),
}
