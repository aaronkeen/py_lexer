/// It should be noted that indentation checks do not verify that mixed
/// spaces and tabs do not depend on the size of a tab stop for correctness.
///
use std::str::{Chars};
use std::iter::{Peekable};

const TAB_STOP_SIZE: u32 = 8;

pub type ResultToken = Result<String, String>;

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
               Some(&c) if is_xid_start(c) =>
                  self.process_identifier(current_line),
               Some(&c) if c.is_digit(10) || c == '.' =>
                  self.process_number(current_line),
               Some(&c) if c == '\\' =>
                  self.process_line_join(current_line),
               Some(_) => (None, Some(current_line)),
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
            None => (Some((0, Ok("** DEDENT **".to_string()))), None),
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

   fn build_identifier(&self, current_line: &mut Line)
      -> (usize, ResultToken)
   {
      let mut token = String::new();
      token.push(current_line.chars.next().unwrap());

      while current_line.chars.peek().is_some() &&
         is_xid_continue(*current_line.chars.peek().unwrap())
      {
         token.push(current_line.chars.next().unwrap());
      }

      (current_line.number, Ok(token))
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
         let result =
            self.build_point_float(Ok(String::new()), line);
         //let result = self.build_exp_float(result, line);
         (line.number, result)
      }
      else
      {
         let result = self.build_decimal_number(line);
         let result = self.build_point_float(result, line);
         //let result = self.build_exp_float(result, line);

         (line.number, result)
      }
   }

   fn build_decimal_number(&self, line: &mut Line<'a>)
      -> ResultToken
   {
      let mut token = String::new();

      token.push(line.chars.next().unwrap());

      while line.chars.peek().is_some() &&
         line.chars.peek().unwrap().is_digit(10)
      {
         token.push(line.chars.next().unwrap());
      }

      Ok(token)
   }

   fn build_zero_prefixed_number(&self, line: &mut Line<'a>)
      -> ResultToken
   {

   }

   fn build_point_float(&self, rtoken: ResultToken, line: &mut Line<'a>)
      -> ResultToken
   {
      if rtoken.is_err()
      {
         return rtoken;
      }

      if line.chars.peek().is_none() ||
         *line.chars.peek().unwrap() != '.'
      {
         return rtoken;
      }

      let mut token = rtoken.ok().unwrap();

      token.push(line.chars.next().unwrap());

      if line.chars.peek().is_some() &&
         line.chars.peek().unwrap().is_digit(10)
      {
         while line.chars.peek().is_some() &&
            line.chars.peek().unwrap().is_digit(10)
         {
            token.push(line.chars.next().unwrap());
         }

         Ok(token)
      }
      else
      {
         Err("** invalid number: ".to_string() + &token)
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
      (Some((current_line.number, Ok("*newline*".to_string()))), None)
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
            (Some((newline.number, Ok("** INDENT **".to_string()))),
               Some(newline))
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
         (Some((current_line.number, Ok("** DEDENT ERROR **".to_string()))),
            Some(current_line))
      }
      else
      {
         self.dedent_count += if self.dedent_count < 0 {1} else {-1};
         (Some((current_line.number, Ok("** DEDENT **".to_string()))),
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

   #[test]
   fn test_identifiers()
   {
      let chars = &mut "abf  \x0C _xyz\n   \n  e2f\n  \tmq3\nn12\\\r\nn3\\ \n  n23\n    n24\n   n25\n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next(), Some((1, Ok("abf".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("_xyz".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((3, Ok("** INDENT **".to_string()))));
      assert_eq!(l.next(), Some((3, Ok("e2f".to_string()))));
      assert_eq!(l.next(), Some((3, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((4, Ok("** INDENT **".to_string()))));
      assert_eq!(l.next(), Some((4, Ok("mq3".to_string()))));
      assert_eq!(l.next(), Some((4, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((5, Ok("** DEDENT **".to_string()))));
      assert_eq!(l.next(), Some((5, Ok("** DEDENT **".to_string()))));
      assert_eq!(l.next(), Some((5, Ok("n12".to_string()))));
      assert_eq!(l.next(), Some((6, Ok("n3".to_string()))));
      assert_eq!(l.next(), Some((6, Err("** bad \\ **".to_string()))));
      assert_eq!(l.next(), Some((6, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("** INDENT **".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("n23".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((8, Ok("** INDENT **".to_string()))));
      assert_eq!(l.next(), Some((8, Ok("n24".to_string()))));
      assert_eq!(l.next(), Some((8, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((9, Ok("** DEDENT ERROR **".to_string()))));
      assert_eq!(l.next(), Some((9, Ok("n25".to_string()))));
      assert_eq!(l.next(), Some((9, Ok("*newline*".to_string()))));
      assert_eq!(l.next(), Some((0, Ok("** DEDENT **".to_string()))));
      assert_eq!(l.next(), Some((0, Ok("** DEDENT **".to_string()))));
   }   

   #[test]
   fn test_numbers()
   {
      let chars = &mut "123 456 45 23.742 23. 12..3 .14 0123.2192 12e17 12e+17 12E-17 00000 00003 0o724 0X32facb7 0b10101010\n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next(), Some((1, Ok("123".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("456".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("45".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("23.742".to_string()))));
      assert_eq!(l.next(), Some((1, Err("** invalid number: 23.".to_string()))));
      assert_eq!(l.next(), Some((1, Err("** invalid number: 12.".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(".3".to_string()))));
      assert_eq!(l.next(), Some((1, Ok(".14".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("0123.2192".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("12e17".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("12e+17".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("12E-17".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("00000".to_string()))));
      assert_eq!(l.next(), Some((1, Err("00003".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("0o724".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("0X32facb7".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("0b10101010".to_string()))));
      assert_eq!(l.next(), Some((1, Ok("*newline*".to_string()))));
   }   

   #[test]
   fn test_dedent()
   {
      let chars = &mut "    abf xyz\n\n\n\n        e2f\n             n12\n  n2\n";
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
      assert_eq!(l.next(), Some((7, Ok("** DEDENT ERROR **".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("n2".to_string()))));
      assert_eq!(l.next(), Some((7, Ok("*newline*".to_string()))));
   }   
}
