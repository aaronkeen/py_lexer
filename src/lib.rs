/// It should be noted that indentation checks do not verify that mixed
/// spaces and tabs do not depend on the size of a tab stop for correctness.
///
use std::str::{Chars};
use std::iter::{Peekable};

const TAB_STOP_SIZE: u32 = 8;

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

   fn next_token(&mut self) -> Option<(usize, String)>
   {
      let current_line = self.current_line.take();
      let result = self.next_token_line(current_line);
      self.current_line = result.1;
      result.0
   }

   fn next_token_line(&mut self, current_line: Option<Line<'a>>)
      -> (Option<(usize, String)>, Option<Line<'a>>)
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
            None => (Some((0, "** DEDENT **".to_string())), None),
            Some(newline) => self.process_line_start(newline)
         }
      }
   }

   fn process_identifier(&self, mut current_line: Line<'a>)
      -> (Option<(usize, String)>, Option<Line<'a>>)
   {
      let result = self.build_identifier(&mut current_line);
      (Some(result), Some(current_line))
   }

   fn build_identifier(&self, current_line: &mut Line) -> (usize, String)
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

   fn consume_space_to_next(&self, current_line: &mut Line)
   {
      while current_line.chars.peek().is_some() &&
         is_space(*current_line.chars.peek().unwrap())
      {
         current_line.chars.next();
      }
   }

   fn process_line_join(&mut self, mut current_line: Line<'a>)
      -> (Option<(usize, String)>, Option<Line<'a>>)
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
         (Some((line_number, "** bad \\ **".to_string())), Some(current_line))
      }
   }

   fn process_newline(&self, current_line: Line<'a>)
      -> (Option<(usize, String)>, Option<Line<'a>>)
   {
      (Some((current_line.number, "*newline*".to_string())), None)
   }

   fn process_line_start(&mut self, mut newline: Line<'a>)
      -> (Option<(usize, String)>, Option<Line<'a>>)
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
            (Some((newline.number, "** INDENT **".to_string())), Some(newline))
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
      -> (Option<(usize, String)>, Option<Line<'a>>)
   {
      if self.dedent_count == -1
      {
         self.dedent_count = 0;
         (Some((current_line.number, "** DEDENT ERROR **".to_string())),
            Some(current_line))
      }
      else
      {
         self.dedent_count += if self.dedent_count < 0 {1} else {-1};
         (Some((current_line.number, "** DEDENT **".to_string())),
            Some(current_line))
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
      let chars = &mut "abf  \x0C _xyz\n   \n  e2f\n  \tmq3\nn12\\\r\nn3\\ \n  n23\n    n24\n   n25\n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "abf")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "_xyz")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((3, "** INDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((3, "e2f")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((3, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((4, "** INDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((4, "mq3")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((4, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((5, "** DEDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((5, "** DEDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((5, "n12")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((6, "n3")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((6, "** bad \\ **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((6, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "** INDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "n23")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((8, "** INDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((8, "n24")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((8, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((9, "** DEDENT ERROR **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((9, "n25")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((9, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((0, "** DEDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((0, "** DEDENT **")));
   }   

   #[test]
   fn test_dedent()
   {
      let chars = &mut "    abf xyz\n\n\n\n        e2f\n             n12\n  n2\n";
      let mut l = Lexer::new(chars.lines_any());
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "** INDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "abf")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "xyz")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((5, "** INDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((5, "e2f")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((5, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((6, "** INDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((6, "n12")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((6, "*newline*")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "** DEDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "** DEDENT **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "** DEDENT ERROR **")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "n2")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((7, "*newline*")));
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
