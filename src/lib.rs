use std::str::{Chars};
use std::iter::{Peekable};

const TAB_STOP_SIZE: u32 = 8;

pub struct Lexer<'a>
{
   indent_stack: Vec<u32>,
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
      let mut iter = (1..).zip(lines)
         .map(|(n, line)| (n, line.chars().peekable()))
         .map(|(n, mut chars)|
            Line::new(n, count_indentation(&mut chars), chars));
      ;
      let first_line = iter.next();

      Lexer{indent_stack: vec![],
         lines: Box::new(iter),
         current_line: first_line,
      }
   }

   fn next_token(&mut self) -> Option<(usize, String)>
   {
      if let Some(mut current_line) = self.current_line.take()
      {
         match current_line.chars.peek()
         {
            Some(&c) if is_xid_start(c) =>
               {
                  let result = self.build_identifier(&mut current_line);
                  self.consume_space_to_next(&mut current_line);
                  self.current_line = Some(current_line);
                  Some(result)
               },
            Some(&c) if c == '\\' =>
               {
                  current_line.chars.next();
                  if current_line.chars.peek().is_none()
                  {
                     // explicit line join
                     self.current_line = self.lines.next();
                     self.next_token()
                  }
                  else
                  {
                     let line_number = current_line.number;
                     self.consume_space_to_next(&mut current_line);
                     self.current_line = Some(current_line);
                     Some((line_number, "** bad \\ **".to_string()))
                  }
               }
            Some(_) => None,
            None =>
               {
                  self.current_line = self.lines.next();
                  // indentation check here
                  Some((current_line.number, "*newline*".to_string()))
               }
         }
      }
      else
      {
         None
      }
   }

   fn build_identifier(&mut self, current_line: &mut Line) -> (usize, String)
   {
      let mut token = String::new();
      token.push(current_line.chars.next().unwrap());

      while current_line.chars.peek().is_some() &&
         is_xid_continue(*current_line.chars.peek().unwrap())
      {
         token.push(current_line.chars.next().unwrap());
      }

      (current_line.number, token)
   }

   fn consume_space_to_next(&mut self, current_line: &mut Line)
   {
      while current_line.chars.peek().is_some() &&
         is_space(*current_line.chars.peek().unwrap())
      {
         current_line.chars.next();
      }
   }
}

impl <'a> Iterator for Lexer<'a>
{
   type Item = (usize, String);

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
      let chars = &mut "abf  \x0C xyz 	e2f  mq3\nn12\\\r\nn3\\ n23\n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "abf")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "xyz")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "e2f")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "mq3")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((2, "n12")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((3, "n3")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((3, "** bad \\ **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((3, "n23")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((3, "*newline*")));
   }   

   /*
   #[test]
   fn test_line_numbers()
   {
      let chars = &mut "abf\n12\r\n3\r23\n".chars() as
         &mut Iterator<Item=char>;
      let mut l = Lexer::new(chars).map(|(n,_)| n);
      assert_eq!(l.next(), Some(1));
      assert_eq!(l.next(), Some(1));
      assert_eq!(l.next(), Some(1));
      assert_eq!(l.next(), Some(2));
      assert_eq!(l.next(), Some(2));
      assert_eq!(l.next(), Some(2));
      assert_eq!(l.next(), Some(3));
      assert_eq!(l.next(), Some(3));
      assert_eq!(l.next(), Some(4));
      assert_eq!(l.next(), Some(4));
      assert_eq!(l.next(), Some(4));
   }   
*/
}
