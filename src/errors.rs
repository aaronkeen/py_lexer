#[derive(Debug, PartialEq, Clone)]
pub enum Error
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

impl Error
{
   pub fn message(self)
      -> String
   {
      match self
      {
         Error::BadLineContinuation => "bad line continuation".to_string(),
         Error::UnterminatedTripleString =>
            "unterminated triple-quoted string".to_string(),
         Error::UnterminatedString => "unterminated string".to_string(),
         Error::InvalidCharacter(c) => format!("invalid character {}", c),
         Error::Dedent => "misaligned dedent".to_string(),
         Error::HexEscapeShort => "missing digits in hex escape".to_string(),
         Error::MalformedUnicodeEscape =>
            "malformed unicode escape".to_string(),
         Error::MalformedNamedUnicodeEscape =>
            "malformed named unicode escape".to_string(),
         Error::UnknownUnicodeName(s) =>
            format!("unknown unicode name '{}'", s),
         Error::MissingDigits => "missing digits".to_string(),
         Error::MalformedFloat =>
            "malformed floating point number".to_string(),
         Error::MalformedImaginary => "malformed imaginary number".to_string(),
         Error::InvalidSymbol(c) => format!("invalid symbol '{}'", c),
         Error::Internal(s) => format!("internal error: {}", s),
      }
   }
}
