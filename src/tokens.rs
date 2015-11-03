#[derive(Debug, PartialEq, Clone)]
pub enum Token
{
   Newline,
   Indent,
   Dedent,
   False,
   None,
   True,
   And,
   As,
   Assert,
   Break,
   Class,
   Continue,
   Def,
   Del,
   Elif,
   Else,
   Except,
   Finally,
   For,
   From,
   Global,
   If,
   Import,
   In,
   Is,
   Lambda,
   Nonlocal,
   Not,
   Or,
   Pass,
   Raise,
   Return,
   Try,
   While,
   With,
   Yield,
   Plus,
   Minus,
   Times,
   Exponent,
   Divide,
   DivideFloor,
   Mod,
   At,
   Lshift,
   Rshift,
   BitAnd,
   BitOr,
   BitXor,
   BitNot,
   LT,
   GT,
   LE,
   GE,
   EQ,
   NE,
   Lparen,
   Rparen,
   Lbracket,
   Rbracket,
   Lbrace,
   Rbrace,
   Comma,
   Colon,
   Dot,
   Ellipsis,
   Semi,
   Arrow,
   Assign,
   AssignPlus,
   AssignMinus,
   AssignTimes,
   AssignDivide,
   AssignDivideFloor,
   AssignMod,
   AssignAt,
   AssignBitAnd,
   AssignBitOr,
   AssignBitXor,
   AssignRshift,
   AssignLshift,
   AssignExponent,
   Quote,
   DoubleQuote,
   Identifier(String),
   String(String),
   Bytes(Vec<u8>),
   DecInteger(String),
   BinInteger(String),
   OctInteger(String),
   HexInteger(String),
   Float(String),
   Imaginary(String),
}

impl Token
{
   pub fn is_decimal_integer(&self)
      -> bool
   {
      match self
      {
         &Token::DecInteger(_) => true,
         _ => false,
      }
   }

   pub fn is_float(&self)
      -> bool
   {
      match self
      {
         &Token::Float(_) => true,
         _ => false,
      }
   }

   pub fn lexeme(self)
      -> String
   {
      match self
      {
         Token::Identifier(s) | Token::String(s) |
            Token::DecInteger(s) | Token::BinInteger(s) |
            Token::OctInteger(s) | Token::HexInteger(s) |
            Token::Float(s) | Token::Imaginary(s) => s,
         Token::Bytes(s) => String::from_utf8(s).unwrap(),
         _ =>
         {
            for &(ref tk, s) in LEXEMES.into_iter()
            {
               if tk == &self
               {
                  return s.to_string();
               }
            }
            unreachable!{};
         }
      }
   }

   pub fn with_equal(&self)
      -> Self
   {
      match self
      {
         &Token::Plus => Token::AssignPlus,
         &Token::Minus => Token::AssignMinus,
         &Token::Times => Token::AssignTimes,
         &Token::Exponent => Token::AssignExponent,
         &Token::Divide => Token::AssignDivide,
         &Token::DivideFloor => Token::AssignDivideFloor,
         &Token::BitAnd => Token::AssignBitAnd,
         &Token::BitOr => Token::AssignBitOr,
         &Token::BitXor => Token::AssignBitXor,
         &Token::Mod => Token::AssignMod,
         &Token::At => Token::AssignAt,
         &Token::Assign => Token::EQ,
         &Token::LT => Token::LE,
         &Token::Lshift => Token::AssignLshift,
         &Token::GT => Token::GE,
         &Token::Rshift => Token::AssignRshift,
         _ => self.clone()
      }
   }
}

pub fn keyword_lookup(token_str: String)
   -> Token
{
   for  &(key, ref tk) in KEYWORDS.into_iter()
   {
      if key == &token_str
      {
         return tk.clone()
      }
   }

   return Token::Identifier(token_str)
}

const KEYWORDS : [(&'static str, Token); 33] =
   [
      ("False", Token::False),
      ("None", Token::None),
      ("True", Token::True),
      ("and", Token::And),
      ("as", Token::As),
      ("assert", Token::Assert),
      ("break", Token::Break),
      ("class", Token::Class),
      ("continue", Token::Continue),
      ("def", Token::Def),
      ("del", Token::Del),
      ("elif", Token::Elif),
      ("else", Token::Else),
      ("except", Token::Except),
      ("finally", Token::Finally),
      ("for", Token::For),
      ("from", Token::From),
      ("global", Token::Global),
      ("if", Token::If),
      ("import", Token::Import),
      ("in", Token::In),
      ("is", Token::Is),
      ("lambda", Token::Lambda),
      ("nonlocal", Token::Nonlocal),
      ("not", Token::Not),
      ("or", Token::Or),
      ("pass", Token::Pass),
      ("raise", Token::Raise),
      ("return", Token::Return),
      ("try", Token::Try),
      ("while", Token::While),
      ("with", Token::With),
      ("yield", Token::Yield),
   ];

const LEXEMES : [(Token, &'static str); 84] =
   [
      (Token::Newline, ""),
      (Token::Indent, ""),
      (Token::Dedent, ""),
      (Token::False, ""),
      (Token::None, ""),
      (Token::True, ""),
      (Token::And, ""),
      (Token::As, ""),
      (Token::Assert, ""),
      (Token::Break, ""),
      (Token::Class, ""),
      (Token::Continue, ""),
      (Token::Def, ""),
      (Token::Del, ""),
      (Token::Elif, ""),
      (Token::Else, ""),
      (Token::Except, ""),
      (Token::Finally, ""),
      (Token::For, ""),
      (Token::From, ""),
      (Token::Global, ""),
      (Token::If, ""),
      (Token::Import, ""),
      (Token::In, ""),
      (Token::Is, ""),
      (Token::Lambda, ""),
      (Token::Nonlocal, ""),
      (Token::Not, ""),
      (Token::Or, ""),
      (Token::Pass, ""),
      (Token::Raise, ""),
      (Token::Return, ""),
      (Token::Try, ""),
      (Token::While, ""),
      (Token::With, ""),
      (Token::Yield, ""),
      (Token::Plus, ""),
      (Token::Minus, ""),
      (Token::Times, ""),
      (Token::Exponent, ""),
      (Token::Divide, ""),
      (Token::DivideFloor, ""),
      (Token::Mod, ""),
      (Token::At, ""),
      (Token::Lshift, ""),
      (Token::Rshift, ""),
      (Token::BitAnd, ""),
      (Token::BitOr, ""),
      (Token::BitXor, ""),
      (Token::BitNot, ""),
      (Token::LT, ""),
      (Token::GT, ""),
      (Token::LE, ""),
      (Token::GE, ""),
      (Token::EQ, ""),
      (Token::NE, ""),
      (Token::Lparen, ""),
      (Token::Rparen, ""),
      (Token::Lbracket, ""),
      (Token::Rbracket, ""),
      (Token::Lbrace, ""),
      (Token::Rbrace, ""),
      (Token::Comma, ""),
      (Token::Colon, ""),
      (Token::Dot, ""),
      (Token::Ellipsis, ""),
      (Token::Semi, ""),
      (Token::Arrow, ""),
      (Token::Assign, ""),
      (Token::AssignPlus, ""),
      (Token::AssignMinus, ""),
      (Token::AssignTimes, ""),
      (Token::AssignDivide, ""),
      (Token::AssignDivideFloor, ""),
      (Token::AssignMod, ""),
      (Token::AssignAt, ""),
      (Token::AssignBitAnd, ""),
      (Token::AssignBitOr, ""),
      (Token::AssignBitXor, ""),
      (Token::AssignRshift, ""),
      (Token::AssignLshift, ""),
      (Token::AssignExponent, ""),
      (Token::Quote, ""),
      (Token::DoubleQuote, ""),
   ];
