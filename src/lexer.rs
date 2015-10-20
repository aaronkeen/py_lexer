/// It should be noted that indentation checks do not verify that mixed
/// spaces and tabs do not depend on the size of a tab stop for correctness.
///
use std::str::Chars;
use std::iter::Peekable;

use tokens::Token;

const TAB_STOP_SIZE: u32 = 8;

pub type ResultToken = Result<Token, String>;

enum InSequence<T>
{
   In(T),
   Out(T)
}

impl <T> InSequence<T>
{
   fn new(t: T) -> Self
   {
      InSequence::In(t)
   }

   fn and_then<F>(self, f: F) -> Self
      where F: FnOnce(T) -> InSequence<T>
   {
      match self
      {
         InSequence::In(t) => f(t),
         InSequence::Out(t) => InSequence::Out(t)
      }
   }

   fn unwrap(self) -> T
   {
      match self
      {
         InSequence::In(t) => t,
         InSequence::Out(t) => t,
      }
   }
}

pub struct Lexer<'a>
{
   indent_stack: Vec<u32>,
   dedent_count: i32,
   lines: Box<Iterator<Item=Line<'a>> + 'a>,
   current_line: Option<Line<'a>>,
}

struct Line<'a>
{
   number: usize,
   indentation: u32,
   chars: Peekable<Chars<'a>>
}

impl <'a> Line<'a>
{
   fn new<'b>(number: usize, indentation: u32, chars: Peekable<Chars<'b>>)
      -> Line<'b>
   {
      Line {number: number, indentation: indentation, chars: chars}
   }
}

impl <'a> Lexer<'a>
{
   fn new<'b, I>(lines: I) -> Lexer<'b>
      where I: Iterator<Item=&'b str> + 'b
   {
      let iter = (1..).zip(lines)
         .map(|(n, line)| (n, line.chars().peekable()))
         .map(|(n, mut chars)|
            Line::new(n, count_indentation(&mut chars), chars));
      ;
      Lexer{indent_stack: vec![0],
         dedent_count: 0,
         lines: Box::new(iter),
         current_line: None,
      }
   }

   fn next_token(&mut self) -> Option<(usize, ResultToken)>
   {
      let current_line = self.current_line.take();
      let result = self.next_token_line(current_line);
      self.current_line = result.1;
      result.0
   }

   fn next_token_line(&mut self, current_line: Option<Line<'a>>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if let Some(mut current_line) = current_line
      {
         if self.dedent_count != 0
         {
            self.process_dedents(current_line)
         }
         else
         {
            self.consume_space_to_next(&mut current_line);
            match current_line.chars.peek()
            {
               Some(&'#') =>
                  self.process_newline(current_line),
               Some(&c) if is_xid_start(c) =>
                  self.process_identifier(current_line),
               Some(&c) if c.is_digit(10) || c == '.' =>
                  self.process_number(current_line),
               Some(&'\\') =>
                  self.process_line_join(current_line),
               Some(_) =>
                  self.process_symbols(current_line),
               None =>
                  self.process_newline(current_line),
            }
         }
      }
      else
      {
         match self.lines.next()
         {
            None if self.indent_stack.len() <= 1 => (None, None),
            None => (Some((0, Ok(Token::Dedent))), None),
            Some(newline) => self.process_line_start(newline)
         }
      }
   }

   fn process_identifier(&self, mut current_line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      let result = self.build_identifier(&mut current_line);
      (Some(result), Some(current_line))
   }

   fn build_identifier(&self, line: &mut Line<'a>)
      -> (usize, ResultToken)
   {
      let token = Token::Identifier(
         self.consume_and_while(String::new(), line, |c| is_xid_continue(c)));
      (line.number, Ok(token))
   }

   fn process_number(&self, mut current_line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      let result = self.build_number(&mut current_line);
      (Some(result), Some(current_line))
   }

   fn build_number(&self, line: &mut Line<'a>) -> (usize, ResultToken)
   {
      let first = *line.chars.peek().unwrap();

      if first == '0'
      {
         let result = self.build_zero_prefixed_number(line);
         (line.number, result)
      }
      else if first == '.'
      {
         let result = self.build_float_part(
            Ok(Token::DecInteger(String::new())), line);
         (line.number, result)
      }
      else
      {
         let result = self.build_decimal_number(line);
         let result = self.build_float_part(result, line);

         (line.number, result)
      }
   }

   fn build_decimal_number(&self, line: &mut Line<'a>)
      -> ResultToken
   {
      self.require_radix_digits(String::new(), line, 10,
         |s| Token::DecInteger(s))
   }

   fn build_zero_prefixed_number(&self, line: &mut Line<'a>)
      -> ResultToken
   {
      let mut token_str = String::new();

      token_str.push(line.chars.next().unwrap());

      match line.chars.peek()
      {
         Some(&'o') | Some(&'O') =>
            {
               token_str.push(line.chars.next().unwrap());
               self.require_radix_digits(token_str, line, 8,
                  |s| Token::OctInteger(s))
            }
         Some(&'x') | Some(&'X') =>
            {
               token_str.push(line.chars.next().unwrap());
               self.require_radix_digits(token_str, line, 16,
                  |s| Token::HexInteger(s))
            }
         Some(&'b') | Some(&'B') =>
            {
               token_str.push(line.chars.next().unwrap());
               self.require_radix_digits(token_str, line, 2,
                  |s| Token::BinInteger(s))
            }
         Some(&'0') => 
            {
               token_str = self.consume_and_while(token_str, line,
                  |c| c.is_digit(1));
               if line.chars.peek().is_some() &&
                  line.chars.peek().unwrap().is_digit(10)
               {
                  let token = self.require_radix_digits(token_str, line, 10,
                     |s| Token::DecInteger(s));
                  self.require_float_part(token, line)
               }
               else
               {
                  self.build_float_part(Ok(Token::DecInteger(token_str)), line)
               }
            }
         Some(&c) if c.is_digit(10) =>
            {
               let token = self.require_radix_digits(token_str, line, 10,
                     |s| Token::DecInteger(s));
               self.require_float_part(token, line)
            }
         _ => self.build_float_part(Ok(Token::DecInteger(token_str)), line),
      }
   }

   fn require_radix_digits<F>(&self, token_str: String, line: &mut Line<'a>,
      radix: u32, token_type: F)
      -> ResultToken
      where F: Fn(String) -> Token
   {
      match line.chars.peek()
      {
         Some(&c) if c.is_digit(radix) =>
            Ok(token_type(
               self.consume_and_while(token_str, line,
               |c| c.is_digit(radix)))),
         _ => Err("** Missing digits: ".to_string() + &token_str)
      }
   }

   fn build_float_part(&self, token: ResultToken, line: &mut Line<'a>)
      -> ResultToken
   {
      let result = self.build_point_float(token, line);
      let result = self.build_exp_float(result, line);
      let result = self.build_img_float(result, line);
      result
   }

   fn require_float_part(&self, token: ResultToken, line: &mut Line<'a>)
      -> ResultToken
   {
      let float_part;

      {
         let first = line.chars.peek();
         float_part = first.is_some() &&
            (*first.unwrap() == '.'
            || *first.unwrap() == 'e' || *first.unwrap() == 'E'
            || *first.unwrap() == 'j' || *first.unwrap() == 'J'
            );
      }

      if !float_part
      {
         Err("** missing float part: ".to_string() +
            &token.ok().unwrap().number_lexeme())
      }
      else
      {
         self.build_float_part(token, line)
      }
   }

   fn build_point_float(&self, token: ResultToken, line: &mut Line<'a>)
      -> ResultToken
   {
      if token.is_err()
      {
         return token;
      }

      if line.chars.peek().is_none() ||
         *line.chars.peek().unwrap() != '.'
      {
         return token;
      }

      match token
      {
         Ok(ref t) if t.is_decimal_integer() => (),
         _ => return Err("Invalid floating point number: ".to_string() +
            &token.ok().unwrap().number_lexeme())
      }

      let mut token_str = token.ok().unwrap().number_lexeme();

      token_str.push(line.chars.next().unwrap());

      if line.chars.peek().is_some() &&
         line.chars.peek().unwrap().is_digit(10)
      {
         self.require_radix_digits(token_str, line, 10, |s| Token::Float(s))
      }
      else if token_str == "."
      {
         Ok(Token::Dot)
      }
      else
      {
         Ok(Token::Float(token_str))
      }
   }

   fn build_exp_float(&self, token: ResultToken, line: &mut Line<'a>)
      -> ResultToken
   {
      if token.is_err()
      {
         return token;
      }

      if line.chars.peek().is_none() ||
         (*line.chars.peek().unwrap() != 'e' &&
         *line.chars.peek().unwrap() != 'E')
      {
         return token;
      }

      match token
      {
         Ok(ref t) if t.is_decimal_integer() || t.is_float() => (),
         _ => return Err(
            format!("Invalid floating point number: {:?}",
               token.ok().unwrap()).to_string()),
      }

      let mut token_str = token.ok().unwrap().number_lexeme();

      token_str.push(line.chars.next().unwrap()); // consume the e|E

      // plus or minus here
      if line.chars.peek().is_some() &&
         (*line.chars.peek().unwrap() == '+' ||
         *line.chars.peek().unwrap() == '-')
      {
         token_str.push(line.chars.next().unwrap()); // consume the +|-
      }

      self.require_radix_digits(token_str, line, 10, |s| Token::Float(s))
   }

   fn build_img_float(&self, token: ResultToken, line: &mut Line<'a>)
      -> ResultToken
   {
      if token.is_err()
      {
         return token;
      }

      if line.chars.peek().is_none() ||
         (*line.chars.peek().unwrap() != 'j' &&
         *line.chars.peek().unwrap() != 'J')
      {
         return token;
      }

      match token
      {
         Ok(ref t) if t.is_decimal_integer() || t.is_float() => (),
         _ => return Err("Invalid imaginary number: ".to_string() +
            &token.ok().unwrap().number_lexeme())
      }

      let mut token_str = token.ok().unwrap().number_lexeme();

      token_str.push(line.chars.next().unwrap()); // consume the j|J

      Ok(Token::Float(token_str))
   }

   fn process_symbols(&self, mut line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      let result = self.build_symbol(&mut line);
      (Some(result), Some(line))
   }

   fn build_symbol(&self, line: &mut Line<'a>)
      -> (usize, ResultToken)
   {
      let result =
         match line.chars.peek()
         {
            Some(&'(') => self.match_one(line, Token::Lparen),
            Some(&')') => self.match_one(line, Token::Rparen),
            Some(&'[') => self.match_one(line, Token::Lbracket),
            Some(&']') => self.match_one(line, Token::Rbracket),
            Some(&'{') => self.match_one(line, Token::Lbrace),
            Some(&'}') => self.match_one(line, Token::Rbrace),
            Some(&',') => self.match_one(line, Token::Comma),
            Some(&':') => self.match_one(line, Token::Colon),
            Some(&';') => self.match_one(line, Token::Semi),
            Some(&'=') => self.start_sequence(Token::Assign, line)
                  .and_then(|t| self.match_seq(t, Token::EQ, line, '='))
                  .unwrap(),
            _ => return (line.number, Err("**Symbol not ready".to_string())),
         };

      (line.number, Ok(result))
   }

   fn match_one(&self, line: &mut Line<'a>, tk: Token)
      -> Token
   {
      line.chars.next();
      tk
   }

   fn start_sequence(&self, token: Token, line: &mut Line<'a>)
      -> InSequence<Token>
   {
      line.chars.next();
      InSequence::In(token)
   }

   fn match_seq(&self, old_token: Token, matched_token: Token,
      line: &mut Line<'a>, c: char)
      -> InSequence<Token>
   {
      if line.chars.peek().is_some() && *line.chars.peek().unwrap() == c
      {
         line.chars.next();
         InSequence::In(matched_token)
      }
      else
      {
         InSequence::Out(old_token)
      }
   }

   fn consume_space_to_next(&self, current_line: &mut Line)
   {
      while current_line.chars.peek().is_some() &&
         is_space(*current_line.chars.peek().unwrap())
      {
         current_line.chars.next();
      }
   }

   fn consume_and_while<P>(&self, mut token_str: String, line: &mut Line<'a>,
      predicate: P)
      -> String
      where P: Fn(char) -> bool
   {
      token_str.push(line.chars.next().unwrap());

      while line.chars.peek().is_some() &&
         predicate(*line.chars.peek().unwrap())
      {
         token_str.push(line.chars.next().unwrap());
      }

      token_str
   }

   fn process_line_join(&mut self, mut current_line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      current_line.chars.next();
      if current_line.chars.peek().is_none()
      {
         // explicit line join
         let newline = self.lines.next();
         self.next_token_line(newline)
      }
      else
      {
         let line_number = current_line.number;
         (Some((line_number, Err("** bad \\ **".to_string()))),
            Some(current_line))
      }
   }

   fn process_newline(&self, current_line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      (Some((current_line.number, Ok(Token::Newline))), None)
   }

   fn process_line_start(&mut self, mut newline: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if let Some(&previous_indent) = self.indent_stack.last()
      {
         if newline.chars.peek().is_none()
         {
            self.next_token_line(None)
         }
         else if newline.indentation > previous_indent
         {
            self.indent_stack.push(newline.indentation);
            (Some((newline.number, Ok(Token::Indent))), Some(newline))
         }
         else if newline.indentation < previous_indent
         {
            let stack_len = self.indent_stack.len();
            let mut i = stack_len - 1;
            while newline.indentation < self.indent_stack[i]
            {
               i -= 1;
            }
            self.indent_stack.truncate(i + 1);
            self.dedent_count = (stack_len - 1 - i) as i32;
            if self.indent_stack[i] != newline.indentation
            {
               self.dedent_count *= -1;
            }
            self.next_token_line(Some(newline))
         }
         else
         {
            self.next_token_line(Some(newline))
         }
      }
      else
      {
         panic!("Internal indentation stack error!")
      }
   }

   fn process_dedents(&mut self, current_line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if self.dedent_count == -1
      {
         self.dedent_count = 0;
         (Some((current_line.number, Err("** DEDENT ERROR **".to_string()))),
            Some(current_line))
      }
      else
      {
         self.dedent_count += if self.dedent_count < 0 {1} else {-1};
         (Some((current_line.number, Ok(Token::Dedent))),
            Some(current_line))
      }
   }
}

impl <'a> Iterator for Lexer<'a>
{
   type Item = (usize, ResultToken);

   fn next(&mut self) -> Option<Self::Item>
   {
      self.next_token()
   }
}

fn determine_spaces(char_count: u32, tab_stop_size: u32) -> u32
{
   tab_stop_size - char_count % tab_stop_size
}

/// This function currently considers \r as a whitespace character instead
/// of an old Mac end of line character.
fn is_space(c: char) -> bool
{
   c == ' ' || c == '\t' || c == '\x0C' || c == '\r' // ignore \r for now
}

fn process_character(count: u32, c: char) -> u32
{
   if c == '\t'
   {
      count + determine_spaces(count, TAB_STOP_SIZE)
   }
   else
   {
      count + 1
   }
}

fn count_indentation(chars: &mut Peekable<Chars>) -> u32
{
   let mut count = 0;

   while let Some(&c) = chars.peek()
   {
      if is_space(c)
      {
         count = process_character(count, c);
         chars.next();
      }
      else
      {
         break;
      }
   }

   count
}

/// This function should be modified to do a more appropriate unicode
/// check.  Eliding for now due to apparently unstable support in Rust.
fn is_xid_start(c: char) -> bool
{
   c.is_alphabetic() || c == '_'
}

/// This function should be modified to do a more appropriate unicode
/// check.  Eliding for now due to apparently unstable support in Rust.
fn is_xid_continue(c: char) -> bool
{
   c.is_alphanumeric() || c == '_'
}


#[cfg(test)]
mod tests
{
   use super::Lexer;
   use tokens::Token;

   #[test]
   fn test_identifiers()
   {
      let chars = "abf  \x0C _xyz\n   \n  e2f\n  \tmq3\nn12\\\r\nn3\\ \n  n23\n    n24\n   n25     # monkey says what?  \n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("abf".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("_xyz".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((3, Ok(Token::Identifier("e2f".to_string())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((4, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((4, Ok(Token::Identifier("mq3".to_string())))));
      assert_eq!(l.next(), Some((4, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((5, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((5, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((5, Ok(Token::Identifier("n12".to_string())))));
      assert_eq!(l.next(), Some((6, Ok(Token::Identifier("n3".to_string())))));
      assert_eq!(l.next(), Some((6, Err("** bad \\ **".to_string()))));
      assert_eq!(l.next(), Some((6, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((7, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((7, Ok(Token::Identifier("n23".to_string())))));
      assert_eq!(l.next(), Some((7, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((8, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((8, Ok(Token::Identifier("n24".to_string())))));
      assert_eq!(l.next(), Some((8, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((9, Err("** DEDENT ERROR **".to_string()))));
      assert_eq!(l.next(), Some((9, Ok(Token::Identifier("n25".to_string())))));
      assert_eq!(l.next(), Some((9, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((0, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((0, Ok(Token::Dedent))));
   }   

   #[test]
   fn test_numbers()
   {
      let chars = "1 123 456 45 23.742 23. 12..3 .14 0123.2192 077e010 12e17 12e+17 12E-17 0 00000 00003 0.2 .e12 0o724 0X32facb7 0b10101010 0x 00000e+00000 79228162514264337593543950336 0xdeadbeef 037j 2.3j 2.j .3j . \n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("1".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("123".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("456".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("45".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("23.742".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("23.".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12.".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float(".3".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float(".14".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("0123.2192".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("077e010".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12e17".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12e+17".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12E-17".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("0".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("00000".to_string())))));
      assert_eq!(l.next(), Some((1, Err("** missing float part: 00003".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("0.2".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Dot))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("e12".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::OctInteger("0o724".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::HexInteger("0X32facb7".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::BinInteger("0b10101010".to_string())))));
      assert_eq!(l.next(), Some((1, Err("** Missing digits: 0x".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("00000e+00000".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("79228162514264337593543950336".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("0xdeadbeef".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary("037j".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary("2.3j".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary("2.j".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary(".3j".to_string())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Dot))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
   }   

/*
   #[test]
   fn test_dedent()
   {
      let chars = "    abf xyz\n\n\n\n        e2f\n             n12\n  n2\n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next(), Some((1, Ok("** INDENT **".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("abf".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("xyz".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((5, Ok("** INDENT **".to_string()))));
      assert_eq!(l.next(), Some((5, Ok("e2f".to_string()))));
      assert_eq!(l.next(), Some((5, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((6, Ok("** INDENT **".to_string()))));
      assert_eq!(l.next(), Some((6, Ok("n12".to_string()))));
      assert_eq!(l.next(), Some((6, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("** DEDENT **".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("** DEDENT **".to_string()))));
      assert_eq!(l.next(), Some((7, Err("** DEDENT ERROR **".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("n2".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("*newline*".to_string()))));
   }   

   #[test]
   fn test_symbols()
   {
      let chars = "(){}[]:,.;===@->+=-=*=/=//=%=@=&=|=^=>>=<<=**=+-*** ///%@<<>>&|^~<><=>===!=...";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next(), Some((1, Ok("(".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(")".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("{".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("}".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("[".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("]".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(":".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(",".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("DOT".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(";".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("==".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("@".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("->".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("+=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("*=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("/=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("//=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("%=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("@=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("&=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("|=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("^=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(">>=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("<<=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("**=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("+".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("-".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("**".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("*".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("//".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("/".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("%".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("@".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("<<".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(">>".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("&".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("|".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("^".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("~".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("<".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(">".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("<=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(">=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("==".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("!=".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("...".to_string()))));
   }   
*/
}
