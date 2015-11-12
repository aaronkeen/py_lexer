#[derive(Debug, PartialEq, Clone)]
pub enum LexerError
{
   BadLineContinuation,
   UnterminatedTripleString,
   UnterminatedString,
   InvalidCharacter(char),
   Dedent,
   HexEscapeShort,
   MalformedUnicodeEscape,
   MalformedNamedUnicodeEscape,
   UnknownUnicodeName(String),
   MissingDigits,
   MalformedFloat,
   MalformedImaginary,
   InvalidSymbol(char),
   Internal(String),
}

impl LexerError
{
   pub fn message(self)
      -> String
   {
      match self
      {
         LexerError::BadLineContinuation =>
            "bad line continuation".to_owned(),
         LexerError::UnterminatedTripleString =>
            "unterminated triple-quoted string".to_owned(),
         LexerError::UnterminatedString => "unterminated string".to_owned(),
         LexerError::InvalidCharacter(c) => format!("invalid character {}", c),
         LexerError::Dedent => "misaligned dedent".to_owned(),
         LexerError::HexEscapeShort =>
            "missing digits in hex escape".to_owned(),
         LexerError::MalformedUnicodeEscape =>
            "malformed unicode escape".to_owned(),
         LexerError::MalformedNamedUnicodeEscape =>
            "malformed named unicode escape".to_owned(),
         LexerError::UnknownUnicodeName(s) =>
            format!("unknown unicode name '{}'", s),
         LexerError::MissingDigits => "missing digits".to_owned(),
         LexerError::MalformedFloat =>
            "malformed floating point number".to_owned(),
         LexerError::MalformedImaginary =>
            "malformed imaginary number".to_owned(),
         LexerError::InvalidSymbol(c) => format!("invalid symbol '{}'", c),
         LexerError::Internal(s) => format!("internal error: {}", s),
      }
   }
}
