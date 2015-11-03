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

