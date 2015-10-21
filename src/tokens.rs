#[derive(Debug, PartialEq)]
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
   AssignMatMul,
   AssignBitAnd,
   AssignBitOr,
   AssignBitXor,
   AssignBitRshift,
   AssignBitLshift,
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
}
