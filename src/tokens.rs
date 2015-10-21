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
   DecInteger(String),
   BinInteger(String),
   OctInteger(String),
   HexInteger(String),
   Float(String),
   Imaginary(String),
}

impl Token
{
   pub fn is_decimal_integer(&self) -> bool
   {
      match self
      {
         &Token::DecInteger(_) => true,
         _ => false,
      }
   }

   pub fn is_float(&self) -> bool
   {
      match self
      {
         &Token::Float(_) => true,
         _ => false,
      }
   }

   pub fn number_lexeme(self) -> String
   {
      match self
      {
         Token::DecInteger(s) | Token::BinInteger(s) |
            Token::OctInteger(s) | Token::HexInteger(s) |
            Token::Float(s) | Token::Imaginary(s) => s,
         _ => panic!(format!("invalid number token: {:?}", self)),
      }
   }

   pub fn with_equal(&self) -> Self
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
