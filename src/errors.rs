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
            "bad line continuation".to_string(),
         LexerError::UnterminatedTripleString =>
            "unterminated triple-quoted string".to_string(),
         LexerError::UnterminatedString => "unterminated string".to_string(),
         LexerError::InvalidCharacter(c) => format!("invalid character {}", c),
         LexerError::Dedent => "misaligned dedent".to_string(),
         LexerError::HexEscapeShort =>
            "missing digits in hex escape".to_string(),
         LexerError::MalformedUnicodeEscape =>
            "malformed unicode escape".to_string(),
         LexerError::MalformedNamedUnicodeEscape =>
            "malformed named unicode escape".to_string(),
         LexerError::UnknownUnicodeName(s) =>
            format!("unknown unicode name '{}'", s),
         LexerError::MissingDigits => "missing digits".to_string(),
         LexerError::MalformedFloat =>
            "malformed floating point number".to_string(),
         LexerError::MalformedImaginary =>
            "malformed imaginary number".to_string(),
         LexerError::InvalidSymbol(c) => format!("invalid symbol '{}'", c),
         LexerError::Internal(s) => format!("internal error: {}", s),
      }
   }
}
