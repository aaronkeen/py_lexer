/// It should be noted that indentation checks do not verify that mixed
/// spaces and tabs do not depend on the size of a tab stop for correctness.

extern crate unicode_names;

use std::ascii::AsciiExt;
use std::char;
use std::cmp;
use std::str::Chars;
use std::iter::Peekable;
use iter::MultiPeekable;
use tokens::{Token, keyword_lookup};
use errors::LexerError;


const TAB_STOP_SIZE: u32 = 8;

pub type ResultToken = Result<Token, LexerError>;

pub struct Lexer<'a>
{
   lexer: Peekable<StringJoiningLexer<'a>>
}

impl <'a> Lexer<'a>
{
   pub fn new<'b, I>(lines: I)
      -> Lexer<'b>
      where I: Iterator<Item=&'b str> + 'b
   {
      Lexer{lexer:
         StringJoiningLexer::new(
            BytesJoiningLexer::new(
               InternalLexer::new(lines)
            )
         ).peekable()}
   }
}

impl <'a> Iterator for Lexer<'a>
{
   type Item = (usize, ResultToken);

   fn next(&mut self)
      -> Option<Self::Item>
   {
      self.lexer.next()
   }
}

pub struct StringJoiningLexer<'a>
{
   lexer: Peekable<BytesJoiningLexer<'a>>
}

impl <'a> StringJoiningLexer<'a>
{
   pub fn new<'b>(lexer: BytesJoiningLexer<'b>)
      -> StringJoiningLexer<'b>
   {
      StringJoiningLexer{lexer: lexer.peekable()}
   }

   fn string_follows(&mut self)
      -> Option<String>
   {
      match self.lexer.peek()
      {
         Some(&(_, Ok(Token::String(_)))) =>
         {
            Some(self.lexer.next().unwrap().1.unwrap().lexeme())
         },
         _ => None,
      }
   }
}

impl <'a> Iterator for StringJoiningLexer<'a>
{
   type Item = (usize, ResultToken);

   fn next(&mut self)
      -> Option<Self::Item>
   {
      match self.lexer.next()
      {
         Some((line_number, Ok(Token::String(s)))) =>
         {
            let mut token_str = s.clone();
            while let Some(follow) = self.string_follows()
            {
               token_str.push_str(&follow)
            }
            Some((line_number, Ok(Token::String(token_str))))
         },
         result => result,
      }
   }
}

pub struct BytesJoiningLexer<'a>
{
   lexer: Peekable<InternalLexer<'a>>
}

impl <'a> BytesJoiningLexer<'a>
{
   pub fn new<'b>(lexer: InternalLexer<'b>)
      -> BytesJoiningLexer<'b>
   {
      BytesJoiningLexer{lexer: lexer.peekable()}
   }

   fn bytes_follows(&mut self)
      -> Option<Vec<u8>>
   {
      match self.lexer.peek()
      {
         Some(&(_, Ok(Token::Bytes(_)))) =>
         {
            match self.lexer.next().unwrap().1.unwrap()
            {
               Token::Bytes(bytes) => Some(bytes),
               _ => unreachable!(),
            }
         },
         _ => None,
      }
   }
}

impl <'a> Iterator for BytesJoiningLexer<'a>
{
   type Item = (usize, ResultToken);

   fn next(&mut self)
      -> Option<Self::Item>
   {
      match self.lexer.next()
      {
         Some((line_number, Ok(Token::Bytes(s)))) =>
         {
            let mut token_vec = s.clone();
            while let Some(mut follow) = self.bytes_follows()
            {
               token_vec.append(&mut follow)
            }
            Some((line_number, Ok(Token::Bytes(token_vec))))
         },
         result => result,
      }
   }
}

struct Line<'a>
{
   number: usize,
   indentation: u32,
   leading_spaces: String,
   chars: MultiPeekable<Chars<'a>>
}

impl <'a> Line<'a>
{
   fn new<'b>(number: usize, indentation: u32, leading_spaces: String,
      chars: MultiPeekable<Chars<'b>>)
      -> Line<'b>
   {
      Line {number: number, indentation: indentation,
         leading_spaces: leading_spaces, chars: chars}
   }
}

pub struct InternalLexer<'a>
{
   indent_stack: Vec<u32>,
   dedent_count: i32,            // negative value to indicate a misalignment
   open_braces: u32,
   lines: Box<Iterator<Item=Line<'a>> + 'a>,
   current_line: Option<Line<'a>>,
}

impl <'a> Iterator for InternalLexer<'a>
{
   type Item = (usize, ResultToken);

   fn next(&mut self)
      -> Option<Self::Item>
   {
      self.next_token()
   }
}

impl <'a> InternalLexer<'a>
{
   pub fn new<'b, I>(lines: I)
      -> InternalLexer<'b>
      where I: Iterator<Item=&'b str> + 'b
   {
      let iter = (1..).zip(lines)
         .map(|(n, line)| (n, MultiPeekable::new(line.chars())))
         .map(|(n, mut chars)|
            {
               let (indentation, spaces) = count_indentation(&mut chars);
               Line::new(n, indentation, spaces, chars)
            });
      ;
      InternalLexer{indent_stack: vec![0],
         dedent_count: 0,
         lines: Box::new(iter),
         current_line: None,
         open_braces: 0,
      }
   }

   fn next_token(&mut self)
      -> Option<(usize, ResultToken)>
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
            consume_space_to_next(&mut current_line);
            match current_line.chars.peek()
            {
               Some(&'#') => self.process_end_of_line(current_line),
               Some(&'u') | Some(&'U') =>
               {
                  if current_line.chars.peek_at(1).map_or(false,
                     |&c| c == '\'' || c == '"')
                  {
                     self.process_string(current_line)
                  }
                  else
                  {
                     process_identifier(current_line)
                  }
               },
               Some(&'r') | Some(&'R') =>
               {
                  if current_line.chars.peek_at(1).map_or(false,
                     |&c| c == '\'' || c == '"')
                  {
                     self.process_string(current_line)
                  }
                  else if current_line.chars.peek_at(1).map_or(false,
                        |&c| c == 'b' || c == 'B') &&
                     current_line.chars.peek_at(2).map_or(false,
                        |&c| c == '\'' || c == '"')
                  {
                     self.process_byte_string(current_line)
                  }
                  else
                  {
                     process_identifier(current_line)
                  }
               },
               Some(&'b') | Some(&'B') =>
               {
                  if current_line.chars.peek_at(1).map_or(false,
                     |&c| c == '\'' || c == '"') ||
                     (current_line.chars.peek_at(1).map_or(false,
                        |&c| c == 'r' || c == 'R') &&
                        current_line.chars.peek_at(2).map_or(false,
                           |&c| c == '\'' || c == '"'))
                  {
                     self.process_byte_string(current_line)
                  }
                  else
                  {
                     process_identifier(current_line)
                  }
               },
               Some(&c) if is_xid_start(c) =>
                  process_identifier(current_line),
               Some(&c) if c.is_digit(10) => process_number(current_line),
               Some(&'.') =>
               {
                  match current_line.chars.peek_at(1)
                  {
                     Some(&c) if c.is_digit(10) => 
                        process_number(current_line),
                     _ => self.process_symbols(current_line),
                  }
               }
               Some(&'\\') => self.process_line_join(current_line),
               Some(&'\'') | Some(&'"') => self.process_string(current_line),
               Some(_) => self.process_symbols(current_line),
               None => self.process_end_of_line(current_line),
            }
         }
      }
      else
      {
         match self.lines.next()
         {
            None if self.indent_stack.len() <= 1 => (None, None),
            None =>
            {
               self.indent_stack.pop();
               (Some((0, Ok(Token::Dedent))), None)
            },
            Some(newline) => self.process_line_start(newline)
         }
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
         (Some((line_number, Err(LexerError::BadLineContinuation))),
            Some(current_line))
      }
   }

   fn process_string(&mut self, mut line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if line.chars.peek().map_or(false, |&c| c == 'u' || c == 'U')
      {
         line.chars.next();   // skip prefix
      }

      let is_raw = if line.chars.peek()
         .map_or(false, |&c| c == 'r' || c == 'R')
         {
            line.chars.next();
            true
         }
         else
         {
            false
         };

      let quote = line.chars.next().unwrap();
      let is_long_string = line.chars.peek().map_or(false, |&c| c == quote) &&
         line.chars.peek_at(1).map_or(false, |&c| c == quote);

      self.build_string(line, quote, is_raw, is_long_string)
   }

   fn process_byte_string(&mut self, mut line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      let is_raw;

      if line.chars.peek_at(1).map_or(false, |&c| c == 'r' || c == 'R') ||
         line.chars.peek().map_or(false, |&c| c == 'r' || c == 'R')
      {
         line.chars.next();   // skip prefix b/B or r/R
         line.chars.next();   // skip prefix b/B or r/R
         is_raw = true;
      }
      else
      {
         line.chars.next();   // skip prefix b/B
         is_raw = false;
      }

      let quote = line.chars.next().unwrap();
      let is_long_string = line.chars.peek().map_or(false, |&c| c == quote) &&
         line.chars.peek_at(1).map_or(false, |&c| c == quote);

      self.build_string_byte(line, quote, is_raw, is_long_string)
   }

   fn build_string(&mut self, mut line: Line<'a>, quote: char,
      is_raw: bool, is_long_string: bool)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if is_long_string
      {
         // consume second two quote characters
         line.chars.next();
         line.chars.next();
      }

      let first_line_number = line.number;
      let mut current_line = line;
      let mut token_str = String::new();

      loop
      {
         match current_line.chars.next()
         {
            Some('\\') =>
            {
               match self.process_escape_sequence(current_line, token_str,
                  is_raw)
               {
                  (_, Ok(new_token_str), Some(new_line)) =>
                  {
                     token_str = new_token_str;
                     current_line = new_line;
                  },
                  (num, Err(msg), new_line) =>
                     return (Some((num, Err(msg))), new_line),
                  (num, Ok(_), None) if is_long_string =>
                     return (Some((num + 1,
                        Err(LexerError::UnterminatedTripleString))), None),
                  (num, Ok(_), None) =>
                     return (Some((num + 1,
                        Err(LexerError::UnterminatedString))), None),
               }
            },
            Some(cur_char) if cur_char == quote =>
            {
               if !is_long_string
               {
                  break;
               }
               else if current_line.chars.peek()
                     .map_or(false, |&c| c == quote) &&
                  current_line.chars.peek_at(1)
                     .map_or(false, |&c| c == quote)
               {
                  // consume closing quotes
                  current_line.chars.next();
                  current_line.chars.next();
                  break;
               }
               else
               {
                  token_str.push(cur_char);
               }
            },
            Some(cur_char) => token_str.push(cur_char),
            None if is_long_string => // end of line
            {
               token_str.push('\n');
               let line_number = current_line.number;
               match self.lines.next()
               {
                  Some(line) =>
                  {
                     token_str.push_str(&line.leading_spaces);
                     current_line = line;
                  },
                  _ => return (Some((line_number + 1,
                        Err(LexerError::UnterminatedTripleString))), None)
               }
            },
            None => return (Some((current_line.number,
                     Err(LexerError::UnterminatedString))),
                     Some(current_line)),
         }
      }

      (Some((first_line_number, Ok(Token::String(token_str)))),
         Some(current_line))
   }

   fn build_string_byte(&mut self, mut line: Line<'a>, quote: char,
      is_raw: bool, is_long_string: bool)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if is_long_string
      {
         // consume second two quote characters
         line.chars.next();
         line.chars.next();
      }

      let first_line_number = line.number;
      let mut current_line = line;
      let mut token_vec = Vec::new();

      loop
      {
         match current_line.chars.next()
         {
            Some('\\') =>
            {
               match self.process_escape_sequence_byte(current_line,
                  token_vec, is_raw)
               {
                  (_, Ok(new_token_vec), Some(new_line)) =>
                  {
                     token_vec = new_token_vec;
                     current_line = new_line;
                  },
                  (num, Err(msg), new_line) =>
                     return (Some((num, Err(msg))), new_line),
                  (num, Ok(_), None) if is_long_string =>
                     return (Some((num + 1,
                        Err(LexerError::UnterminatedTripleString))), None),
                  (num, Ok(_), None) =>
                     return (Some((num + 1,
                        Err(LexerError::UnterminatedString))), None),
               }
            },
            Some(cur_char) if cur_char == quote =>
            {
               if !is_long_string
               {
                  break;
               }
               else if current_line.chars.peek()
                     .map_or(false, |&c| c == quote) &&
                  current_line.chars.peek_at(1)
                     .map_or(false, |&c| c == quote)
               {
                  // consume closing quotes
                  current_line.chars.next();
                  current_line.chars.next();
                  break;
               }
               else
               {
                  token_vec.push(cur_char as u8);
               }
            },
            Some(cur_char) => token_vec.push(cur_char as u8),
            None if is_long_string => // end of line
            {
               token_vec.push('\n' as u8);
               let line_number = current_line.number;
               match self.lines.next()
               {
                  Some(line) =>
                  {
                     for c in line.leading_spaces.chars()
                     {
                        token_vec.push(c as u8);
                     }
                     current_line = line;
                  },
                  _ => return (Some((line_number + 1,
                        Err(LexerError::UnterminatedTripleString))), None)
               }
            },
            None => return (Some((current_line.number,
                     Err(LexerError::UnterminatedString))), Some(current_line)),
         }
      }

      (Some((first_line_number, Ok(Token::Bytes(token_vec)))),
         Some(current_line))
   }

   fn process_escape_sequence(&mut self, current_line: Line<'a>,
      token_str: String, is_raw: bool)
      -> (usize, Result<String, LexerError>, Option<Line<'a>>)
   {
      let line_number = current_line.number;
      let (new_line, new_token_res) =
         self.handle_escaped_character(current_line, token_str, is_raw);

      return (line_number, new_token_res, new_line)
   }

   fn handle_escaped_character(&mut self, mut line: Line<'a>,
      mut token_str: String, is_raw: bool)
      -> (Option<Line<'a>>, Result<String,LexerError>)
   {
      match line.chars.next()
      {
         None => // end of line escape, join lines
         {
            if is_raw
            {
               token_str.push('\\');
               token_str.push('\n');
            }
            let next_line = self.lines.next();
            next_line.as_ref()
               .map(|l| token_str.push_str(&l.leading_spaces));
            (next_line, Ok(token_str))
         },
         Some('\\') if !is_raw =>
         {
            token_str.push('\\');
            (Some(line), Ok(token_str))
         },
         Some('\'') if !is_raw =>
         {
            token_str.push('\'');
            (Some(line), Ok(token_str))
         },
         Some('"') if !is_raw =>
         {
            token_str.push('"');
            (Some(line), Ok(token_str))
         },
         Some('a') if !is_raw =>
         {
            token_str.push('\x07'); // BEL
            (Some(line), Ok(token_str))
         },
         Some('b') if !is_raw =>
         {
            token_str.push('\x08'); // BS
            (Some(line), Ok(token_str))
         },
         Some('f') if !is_raw =>
         {
            token_str.push('\x0C'); // FF
            (Some(line), Ok(token_str))
         },
         Some('n') if !is_raw =>
         {
            token_str.push('\n');
            (Some(line), Ok(token_str))
         },
         Some('r') if !is_raw =>
         {
            token_str.push('\r');
            (Some(line), Ok(token_str))
         },
         Some('t') if !is_raw =>
         {
            token_str.push('\t');
            (Some(line), Ok(token_str))
         },
         Some('v') if !is_raw =>
         {
            token_str.push('\x0B'); // VT
            (Some(line), Ok(token_str))
         },
         Some(c) if c.is_digit(8) && !is_raw =>   // octal
         {
            let (new_line, token_res) =
               push_octal_character(line, token_str, c);
            (new_line, token_res)
         },
         Some('x') if !is_raw =>
         {
            let (new_line, token_res) =
               push_hex_character(line, token_str);
            (new_line, token_res)
         },
         Some('N') if !is_raw =>
         {
            let (new_line, token_res) =
               push_named_character(line, token_str);
            (new_line, token_res)
         },
         Some('u') if !is_raw =>
         {
            let (new_line, token_res) =
               push_unicode_character(line, token_str, 4);
            (new_line, token_res)
         },
         Some('U') if !is_raw =>
         {
            let (new_line, token_res) =
               push_unicode_character(line, token_str, 8);
            (new_line, token_res)
         },
         Some(c) =>
         {
            token_str.push('\\');
            token_str.push(c);
            (Some(line), Ok(token_str))
         },
      }
   }

   fn process_escape_sequence_byte(&mut self, current_line: Line<'a>,
      token_vec: Vec<u8>, is_raw: bool)
      -> (usize, Result<Vec<u8>, LexerError>, Option<Line<'a>>)
   {
      let line_number = current_line.number;
      let (new_line, new_token_res) =
         self.handle_escaped_character_byte(current_line, token_vec, is_raw);

      return (line_number, new_token_res, new_line)
   }

   fn handle_escaped_character_byte(&mut self, mut line: Line<'a>,
      mut token_vec: Vec<u8>, is_raw: bool)
      -> (Option<Line<'a>>, Result<Vec<u8>, LexerError>)
   {
      match line.chars.next()
      {
         None => // end of line escape, join lines
         {
            if is_raw
            {
               token_vec.push('\\' as u8);
               token_vec.push('\n' as u8);
            }
            let next_line = self.lines.next();
            next_line.as_ref()
               .map(|l| {
                  for c in l.leading_spaces.chars()
                  {
                     token_vec.push(c as u8)
                  }
               });
            (next_line, Ok(token_vec))
         },
         Some('\\') if !is_raw =>
         {
            token_vec.push('\\' as u8);
            (Some(line), Ok(token_vec))
         },
         Some('\'') if !is_raw =>
         {
            token_vec.push('\'' as u8);
            (Some(line), Ok(token_vec))
         },
         Some('"') if !is_raw =>
         {
            token_vec.push('"' as u8);
            (Some(line), Ok(token_vec))
         },
         Some('a') if !is_raw =>
         {
            token_vec.push('\x07' as u8); // BEL
            (Some(line), Ok(token_vec))
         },
         Some('b') if !is_raw =>
         {
            token_vec.push('\x08' as u8); // BS
            (Some(line), Ok(token_vec))
         },
         Some('f') if !is_raw =>
         {
            token_vec.push('\x0C' as u8); // FF
            (Some(line), Ok(token_vec))
         },
         Some('n') if !is_raw =>
         {
            token_vec.push('\n' as u8);
            (Some(line), Ok(token_vec))
         },
         Some('r') if !is_raw =>
         {
            token_vec.push('\r' as u8);
            (Some(line), Ok(token_vec))
         },
         Some('t') if !is_raw =>
         {
            token_vec.push('\t' as u8);
            (Some(line), Ok(token_vec))
         },
         Some('v') if !is_raw =>
         {
            token_vec.push('\x0B' as u8); // VT
            (Some(line), Ok(token_vec))
         },
         Some(c) if c.is_digit(8) && !is_raw =>   // octal
         {
            let (new_line, token_res) =
               push_octal_character_byte(line, token_vec, c);
            (new_line, token_res)
         },
         Some('x') if !is_raw =>
         {
            let (new_line, token_res) =
               push_hex_character_byte(line, token_vec);
            (new_line, token_res)
         },
         Some(c) if c.is_ascii() =>
         {
            token_vec.push('\\' as u8);
            token_vec.push(c as u8);
            (Some(line), Ok(token_vec))
         },
         Some(c) => (Some(line), Err(LexerError::InvalidCharacter(c))),
      }
   }

   fn process_line_start(&mut self, mut newline: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if let Some(&previous_indent) = self.indent_stack.last()
      {
         if newline.chars.peek().is_none() ||                  // blank line
            newline.chars.peek().map_or(false, |&c| c == '#')  // only comment
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
               self.dedent_count *= -1;         // negate to flag error
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
         (Some((current_line.number, Err(LexerError::Dedent))),
            Some(current_line))
      }
      else
      {
         self.dedent_count += if self.dedent_count < 0 {1} else {-1};
         (Some((current_line.number, Ok(Token::Dedent))), Some(current_line))
      }
   }

   fn process_symbols(&mut self, mut line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      let result = self.build_symbol(&mut line);
      (Some(result), Some(line))
   }

   fn build_symbol(&mut self, line: &mut Line<'a>)
      -> (usize, ResultToken)
   {
      let result =
         match line.chars.peek()
         {
            Some(&'(') =>
            {
               self.open_braces += 1;
               match_one(line, Token::Lparen)
            },
            Some(&')') =>
            {
               self.open_braces = cmp::max(0, self.open_braces - 1);
               match_one(line, Token::Rparen)
            },
            Some(&'[') =>
            {
               self.open_braces += 1;
               match_one(line, Token::Lbracket)
            },
            Some(&']') =>
            {
               self.open_braces = cmp::max(0, self.open_braces - 1);
               match_one(line, Token::Rbracket)
            },
            Some(&'{') =>
            {
               self.open_braces += 1;
               match_one(line, Token::Lbrace)
            },
            Some(&'}') =>
            {
               self.open_braces = cmp::max(0, self.open_braces - 1);
               match_one(line, Token::Rbrace)
            },
            Some(&',') => match_one(line, Token::Comma),
            Some(&':') => match_one(line, Token::Colon),
            Some(&';') => match_one(line, Token::Semi),
            Some(&'~') => match_one(line, Token::BitNot),
            Some(&'=') => match_pair_opt(
               match_one(line, Token::Assign), line, '=', Token::EQ),
            Some(&'@') => match_pair_opt(
               match_one(line, Token::At), line, '=', Token::AssignAt),
            Some(&'%') => match_pair_opt(
               match_one(line, Token::Mod), line, '=', Token::AssignMod),
            Some(&'&') => match_pair_opt(
               match_one(line, Token::BitAnd), line, '=', Token::AssignBitAnd),
            Some(&'|') => match_pair_opt(
               match_one(line, Token::BitOr), line, '=', Token::AssignBitOr),
            Some(&'^') => match_pair_opt(
               match_one(line, Token::BitXor), line, '=', Token::AssignBitXor),
            Some(&'+') => match_pair_opt(
               match_one(line, Token::Plus), line, '=', Token::AssignPlus),
            Some(&'*') =>
            {
               let token = match_one(line, Token::Times);
               match_pair_eq_opt(line, token, '*', Token::Exponent)
            },
            Some(&'/') =>
            {
               let token = match_one(line, Token::Divide);
               match_pair_eq_opt(line, token, '/', Token::DivideFloor)
            },
            Some(&'<') =>
            {
               let token = match_one(line, Token::LT);
               match_pair_eq_opt(line, token, '<', Token::Lshift)
            },
            Some(&'>') =>
            {
               let token = match_one(line, Token::GT);
               match_pair_eq_opt(line, token, '>', Token::Rshift)
            },
            Some(&'-') =>
            {
               let token = match_one(line, Token::Minus);
               let token = match_pair_opt(token, line, '=', Token::AssignMinus);
               if token == Token::Minus
               {
                  match_pair_opt(token, line, '>', Token::Arrow)
               }
               else
               {
                  token
               }
            },
            Some(&'!') =>
            {
               // consume character
               line.chars.next();
               match line.chars.peek()
               {
                  Some(&'=') => match_one(line, Token::NE),
                  _ => return (line.number,
                        Err(LexerError::InvalidSymbol('!'))),
               }
            }
            Some(&'.') =>
            {
               // consume character
               line.chars.next();
               match line.chars.peek()
               {
                  Some(&'.') => match line.chars.peek_at(1)
                  {
                     Some(&'.') =>
                     {
                        line.chars.next();
                        line.chars.next();
                        Token::Ellipsis
                     },
                     _ => Token::Dot,
                  },
                  _ => Token::Dot, 
               }
            }
            Some(&c) => return (line.number, Err(LexerError::InvalidSymbol(c))),
            _ => return (line.number,
               Err(LexerError::Internal(
                  "error processing symbol".to_owned()))),
         };

      (line.number, Ok(result))
   }

   fn process_end_of_line(&mut self, line: Line<'a>)
      -> (Option<(usize, ResultToken)>, Option<Line<'a>>)
   {
      if self.open_braces == 0
      {
         (Some((line.number, Ok(Token::Newline))), None)
      }
      else
      {
         let newline = self.lines.next();
         self.next_token_line(newline)
      }
   }
}

fn process_identifier(mut current_line: Line)
   -> (Option<(usize, ResultToken)>, Option<Line>)
{
   let result = build_identifier(&mut current_line);
   (Some(result), Some(current_line))
}

fn build_identifier(line: &mut Line)
   -> (usize, ResultToken)
{
   let token = keyword_lookup(
      consume_and_while(String::new(), line, |c| is_xid_continue(c)));
   (line.number, Ok(token))
}

fn process_number(mut current_line: Line)
   -> (Option<(usize, ResultToken)>, Option<Line>)
{
   let result = build_number(&mut current_line);
   (Some(result), Some(current_line))
}

fn build_number(line: &mut Line)
   -> (usize, ResultToken)
{
   match line.chars.peek()
   {
      Some(&'0') =>
      {
         let result = build_zero_prefixed_number(line);
         (line.number, result)
      },
      Some(&'.') =>
      {
         let result = build_dot_prefixed(line);
         (line.number, result)
      },
      _ =>
      {
         let result = build_decimal_number(line);
         let result = build_float_part(result, line);

         (line.number, result)
      },
   }
}

fn build_dot_prefixed(line: &mut Line)
   -> ResultToken
{
   let mut token_str = String::new();
   token_str.push(line.chars.next().unwrap());

   match line.chars.peek()
   {
      Some(&c) if c.is_digit(10) =>
      {
         let result = require_radix_digits(token_str, line, 10,
            |s| Token::Float(s));
         let result = build_exp_float(result, line);
         let result = build_img_float(result, line);
         result
      },
      _ => Err(LexerError::Internal("dot".to_owned()))
   }
}

fn build_decimal_number(line: &mut Line)
   -> ResultToken
{
   require_radix_digits(String::new(), line, 10, |s| Token::DecInteger(s))
}

fn build_zero_prefixed_number(line: &mut Line)
   -> ResultToken
{
   let mut token_str = String::new();
   token_str.push(line.chars.next().unwrap());

   match line.chars.peek()
   {
      Some(&'o') | Some(&'O') =>
      {
         token_str.push(line.chars.next().unwrap());
         require_radix_digits(token_str, line, 8, |s| Token::OctInteger(s))
      },
      Some(&'x') | Some(&'X') =>
      {
         token_str.push(line.chars.next().unwrap());
         require_radix_digits(token_str, line, 16, |s| Token::HexInteger(s))
      },
      Some(&'b') | Some(&'B') =>
      {
         token_str.push(line.chars.next().unwrap());
         require_radix_digits(token_str, line, 2, |s| Token::BinInteger(s))
      },
      Some(&'0') => 
      {
         token_str = consume_and_while(token_str, line,
            |c| c.is_digit(1));
         if line.chars.peek().map_or(false, |c| c.is_digit(10))
         {
            let token = require_radix_digits(token_str, line, 10,
               |s| Token::DecInteger(s));
            require_float_part(token, line)
         }
         else
         {
            build_float_part(Ok(Token::DecInteger(token_str)), line)
         }
      },
      Some(&c) if c.is_digit(10) =>
      {
         let token = require_radix_digits(token_str, line, 10,
               |s| Token::DecInteger(s));
         require_float_part(token, line)
      },
      _ => build_float_part(Ok(Token::DecInteger(token_str)), line),
   }
}

fn push_octal_character(mut line: Line, mut token_str: String, c: char)
   -> (Option<Line>, Result<String, LexerError>)
{
   let mut octal = String::new();
   octal.push(c);
   if line.chars.peek().map_or(false, |&c| c.is_digit(8))
   {
      octal.push(line.chars.next().unwrap());
      if line.chars.peek().map_or(false, |&c| c.is_digit(8))
      {
         octal.push(line.chars.next().unwrap());
      }
   }
   match u32::from_str_radix(&octal, 8)
   {
      Ok(v) =>
      {
         token_str.push(char::from_u32(v).unwrap());
         (Some(line), Ok(token_str))
      },
      _ => unreachable!(),
   }
}

fn push_hex_character(mut line: Line, mut token_str: String)
   -> (Option<Line>, Result<String, LexerError>)
{
   match build_n_while(&mut line, 2, |c| c.is_digit(16))
   {
      Some(s) =>
      {
         let v = u32::from_str_radix(&s, 16).unwrap();
         token_str.push(char::from_u32(v).unwrap());
         (Some(line), Ok(token_str))
      },
      None => (Some(line), Err(LexerError::HexEscapeShort)),
   }
}

fn push_unicode_character(mut line: Line, mut token_str: String,
   num_digits: u32)
   -> (Option<Line>, Result<String, LexerError>)
{
   match build_n_while(&mut line, num_digits, |c| c.is_digit(16))
   {
      Some(s) =>
      {
         let v = u32::from_str_radix(&s, 16).unwrap();
         token_str.push(char::from_u32(v).unwrap());
         (Some(line), Ok(token_str))
      },
      None => (Some(line), Err(LexerError::MalformedUnicodeEscape)),
   }
}

fn push_named_character(mut line: Line, mut token_str: String)
   -> (Option<Line>, Result<String, LexerError>)
{
   if !line.chars.peek().map_or(false, |&c| c == '{')
   {
      return (Some(line), Err(LexerError::MalformedNamedUnicodeEscape));
   }

   line.chars.next();   // consume {

   let mut named = String::new();
   while line.chars.peek().map_or(false, |&c| c != '}')
   {
      named.push(line.chars.next().unwrap());
   }

   if !line.chars.peek().map_or(false, |&c| c == '}')
   {
      return (Some(line), Err(LexerError::MalformedNamedUnicodeEscape));
   }

   line.chars.next(); // consume }
   match unicode_names::character(&named)
   {
      Some(c) =>
      {
         token_str.push(c);
         (Some(line), Ok(token_str))
      },
      _ => (Some(line), Err(LexerError::UnknownUnicodeName(named))),
   }
}

fn push_octal_character_byte(mut line: Line, mut token_vec: Vec<u8>, c: char)
   -> (Option<Line>, Result<Vec<u8>, LexerError>)
{
   let mut octal = String::new();
   octal.push(c);
   if line.chars.peek().map_or(false, |&c| c.is_digit(8))
   {
      octal.push(line.chars.next().unwrap());
      if line.chars.peek().map_or(false, |&c| c.is_digit(8))
      {
         octal.push(line.chars.next().unwrap());
      }
   }
   match u32::from_str_radix(&octal, 8)
   {
      Ok(v) =>
      {
         token_vec.push(v as u8);
         (Some(line), Ok(token_vec))
      },
      _ => unreachable!(),
   }
}

fn push_hex_character_byte(mut line: Line, mut token_vec: Vec<u8>)
   -> (Option<Line>, Result<Vec<u8>, LexerError>)
{
   match build_n_while(&mut line, 2, |c| c.is_digit(16))
   {
      Some(s) =>
      {
         let v = u32::from_str_radix(&s, 16).unwrap();
         token_vec.push(v as u8);
         (Some(line), Ok(token_vec))
      }
      None => (Some(line), Err(LexerError::HexEscapeShort))
   }
}

fn build_n_while<P>(line: &mut Line, n: u32, predicate: P)
   -> Option<String>
   where P: Fn(char) -> bool
{
   let mut s = String::new();
   let mut i = 0;

   while i < n && line.chars.peek().map_or(false, |&c| predicate(c))
   {
      s.push(line.chars.next().unwrap());
      i += 1;
   }

   if i == n { Some(s) } else { None }
}

fn require_radix_digits<F>(token_str: String, line: &mut Line,
   radix: u32, token_type: F)
   -> ResultToken
      where F: Fn(String) -> Token
{
   match line.chars.peek()
   {
      Some(&c) if c.is_digit(radix) =>
         Ok(token_type(consume_and_while(token_str, line,
            |c| c.is_digit(radix)))),
      _ => Err(LexerError::MissingDigits)
   }
}

fn build_float_part(token: ResultToken, line: &mut Line)
   -> ResultToken
{
   let result = build_point_float(token, line);
   let result = build_exp_float(result, line);
   let result = build_img_float(result, line);
   result
}

fn require_float_part(token: ResultToken, line: &mut Line)
   -> ResultToken
{
   let float_part;

   {
      let first = line.chars.peek();
      float_part = first.map_or(false,
         |&c| c == '.' || c == 'e' || c == 'E' || c == 'j' || c == 'J'
         );
   }

   if !float_part
   {
      Err(LexerError::MalformedFloat)
   }
   else
   {
      build_float_part(token, line)
   }
}

fn build_point_float(token: ResultToken, line: &mut Line)
   -> ResultToken
{
   if token.is_err()
   {
      return token;
   }

   if line.chars.peek().map_or(true, |&c| c != '.')
   {
      return token;
   }

   match token
   {
      Ok(ref t) if t.is_decimal_integer() => (),
      _ => return Err(LexerError::MalformedFloat)
   }

   let mut token_str = token.unwrap().lexeme();

   token_str.push(line.chars.next().unwrap());

   if line.chars.peek().map_or(false, |c| c.is_digit(10))
   {
      require_radix_digits(token_str, line, 10, |s| Token::Float(s))
   }
   else
   {
      Ok(Token::Float(token_str))
   }
}

fn build_exp_float(token: ResultToken, line: &mut Line)
   -> ResultToken
{
   if token.is_err()
   {
      return token;
   }

   if line.chars.peek().map_or(true, |&c| c != 'e' && c != 'E')
   {
      return token;
   }

   match token
   {
      Ok(ref t) if t.is_decimal_integer() || t.is_float() => (),
      _ => return Err(LexerError::MalformedFloat)
   }

   let mut token_str = token.unwrap().lexeme();

   token_str.push(line.chars.next().unwrap()); // consume the e|E

   // plus or minus here
   if line.chars.peek().map_or(false, |&c| c == '+' || c == '-')
   {
      token_str.push(line.chars.next().unwrap()); // consume the +|-
   }

   require_radix_digits(token_str, line, 10, |s| Token::Float(s))
}

fn build_img_float(token: ResultToken, line: &mut Line)
   -> ResultToken
{
   if token.is_err()
   {
      return token;
   }

   if line.chars.peek().map_or(true, |&c| c != 'j' && c != 'J')
   {
      return token;
   }

   match token
   {
      Ok(ref t) if t.is_decimal_integer() || t.is_float() => (),
      _ => return Err(LexerError::MalformedImaginary)
   }

   let mut token_str = token.unwrap().lexeme();

   token_str.push(line.chars.next().unwrap()); // consume the j|J

   Ok(Token::Imaginary(token_str))
}

fn match_one(line: &mut Line, tk: Token)
   -> Token
{
   line.chars.next();
   tk
}

fn match_pair_opt(old_token: Token, line: &mut Line,
   c: char, matched_token: Token)
   -> Token
{
   if line.chars.peek().map_or(false, |&k| k == c)
   {
      line.chars.next();
      matched_token
   }
   else
   {
      old_token
   }
}

fn match_pair_eq_opt(line: &mut Line, initial_token: Token,
   paired_char: char, paired_token: Token)
   -> Token
{
   let token = match_pair_opt(initial_token, line, paired_char, paired_token);
   let weq = token.with_equal();
   match_pair_opt(token, line, '=', weq)
}

fn consume_space_to_next(current_line: &mut Line)
{
   while current_line.chars.peek().map_or(false, |&c| is_space(c))
   {
      current_line.chars.next();
   }
}

fn consume_and_while<P>(mut token_str: String, line: &mut Line, predicate: P)
   -> String
      where P: Fn(char) -> bool
{
   token_str.push(line.chars.next().unwrap());

   while line.chars.peek().map_or(false, |&c| predicate(c))
   {
      token_str.push(line.chars.next().unwrap());
   }

   token_str
}

fn determine_spaces(char_count: u32, tab_stop_size: u32)
   -> u32
{
   tab_stop_size - char_count % tab_stop_size
}

/// This function currently considers \r as a whitespace character instead
/// of an old Mac end of line character.
fn is_space(c: char)
   -> bool
{
   c == ' ' || c == '\t' || c == '\x0C' || c == '\r' // ignore \r for now
}

fn process_character(count: u32, c: char)
   -> u32
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

fn count_indentation(chars: &mut MultiPeekable<Chars>)
   -> (u32, String)
{
   let mut count = 0;
   let mut spaces = String::new();

   while let Some(&c) = chars.peek()
   {
      if is_space(c)
      {
         count = process_character(count, c);
         spaces.push(chars.next().unwrap());
      }
      else
      {
         break;
      }
   }

   (count, spaces)
}

/// This function should be modified to do a more appropriate unicode
/// check.  Eliding for now due to apparently unstable support in Rust.
fn is_xid_start(c: char)
   -> bool
{
   c.is_alphabetic() || c == '_'
}

/// This function should be modified to do a more appropriate unicode
/// check.  Eliding for now due to apparently unstable support in Rust.
fn is_xid_continue(c: char)
   -> bool
{
   c.is_alphanumeric() || c == '_'
}



/*
   -----------------------------------------------------------------
   ----------------------- TESTS BELOW HERE ------------------------
   -----------------------------------------------------------------
*/

#[cfg(test)]
mod tests
{
   use super::Lexer;
   use tokens::Token;
   use errors::LexerError;

   #[test]
   fn test_identifiers()
   {
      let chars = "abf  \x0C _xyz\n   \n  e2f\n  \tmq3\nn12\\\r\nn3\\ \n  n23\n    n24\n   n25     # monkey says what?  \n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("abf".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("_xyz".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((3, Ok(Token::Identifier("e2f".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((4, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((4, Ok(Token::Identifier("mq3".to_owned())))));
      assert_eq!(l.next(), Some((4, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((5, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((5, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((5, Ok(Token::Identifier("n12".to_owned())))));
      assert_eq!(l.next(), Some((6, Ok(Token::Identifier("n3".to_owned())))));
      assert_eq!(l.next(), Some((6, Err(LexerError::BadLineContinuation))));
      assert_eq!(l.next(), Some((6, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((7, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((7, Ok(Token::Identifier("n23".to_owned())))));
      assert_eq!(l.next(), Some((7, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((8, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((8, Ok(Token::Identifier("n24".to_owned())))));
      assert_eq!(l.next(), Some((8, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((9, Err(LexerError::Dedent))));
      assert_eq!(l.next(), Some((9, Ok(Token::Identifier("n25".to_owned())))));
      assert_eq!(l.next(), Some((9, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((0, Ok(Token::Dedent))));
   }   

   #[test]
   fn test_numbers()
   {
      let chars = "1 123 456 45 23.742 23. 12..3 .14 0123.2192 077e010 12e17 12e+17 12E-17 0 00000 00003 0.2 .e12 0o724 0X32facb7 0b10101010 0x 00000e+00000 79228162514264337593543950336 0xdeadbeef 037j 2.3j 2.j .3j . 3..2\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("1".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("123".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("456".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("45".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("23.742".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("23.".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12.".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float(".3".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float(".14".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("0123.2192".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("077e010".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12e17".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12e+17".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("12E-17".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("0".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("00000".to_owned())))));
      assert_eq!(l.next(), Some((1, Err(LexerError::MalformedFloat))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("0.2".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Dot))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("e12".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::OctInteger("0o724".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::HexInteger("0X32facb7".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::BinInteger("0b10101010".to_owned())))));
      assert_eq!(l.next(), Some((1, Err(LexerError::MissingDigits))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("00000e+00000".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("79228162514264337593543950336".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::HexInteger("0xdeadbeef".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary("037j".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary("2.3j".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary("2.j".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Imaginary(".3j".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Dot))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float("3.".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Float(".2".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
   }   

   #[test]
   fn test_dedent()
   {
      let chars = "    abf xyz\n\n\n\n        e2f\n             n12\n  n2\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("abf".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("xyz".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((5, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((5, Ok(Token::Identifier("e2f".to_owned())))));
      assert_eq!(l.next(), Some((5, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((6, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((6, Ok(Token::Identifier("n12".to_owned())))));
      assert_eq!(l.next(), Some((6, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((7, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((7, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((7, Err(LexerError::Dedent))));
      assert_eq!(l.next(), Some((7, Ok(Token::Identifier("n2".to_owned())))));
      assert_eq!(l.next(), Some((7, Ok(Token::Newline))));
   }   

   #[test]
   fn test_symbols()
   {
      let chars = "(){}[]:,.;..===@->+=-=*=/=//=%=@=&=|=^=>>=<<=**=+-***///%@<<>>&|^~<><=>===!=!...";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Lparen))));
      assert_eq!(l.next(), Some((1, Ok(Token::Rparen))));
      assert_eq!(l.next(), Some((1, Ok(Token::Lbrace))));
      assert_eq!(l.next(), Some((1, Ok(Token::Rbrace))));
      assert_eq!(l.next(), Some((1, Ok(Token::Lbracket))));
      assert_eq!(l.next(), Some((1, Ok(Token::Rbracket))));
      assert_eq!(l.next(), Some((1, Ok(Token::Colon))));
      assert_eq!(l.next(), Some((1, Ok(Token::Comma))));
      assert_eq!(l.next(), Some((1, Ok(Token::Dot))));
      assert_eq!(l.next(), Some((1, Ok(Token::Semi))));
      assert_eq!(l.next(), Some((1, Ok(Token::Dot))));
      assert_eq!(l.next(), Some((1, Ok(Token::Dot))));
      assert_eq!(l.next(), Some((1, Ok(Token::EQ))));
      assert_eq!(l.next(), Some((1, Ok(Token::Assign))));
      assert_eq!(l.next(), Some((1, Ok(Token::At))));
      assert_eq!(l.next(), Some((1, Ok(Token::Arrow))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignPlus))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignMinus))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignTimes))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignDivide))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignDivideFloor))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignMod))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignAt))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignBitAnd))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignBitOr))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignBitXor))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignRshift))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignLshift))));
      assert_eq!(l.next(), Some((1, Ok(Token::AssignExponent))));
      assert_eq!(l.next(), Some((1, Ok(Token::Plus))));
      assert_eq!(l.next(), Some((1, Ok(Token::Minus))));
      assert_eq!(l.next(), Some((1, Ok(Token::Exponent))));
      assert_eq!(l.next(), Some((1, Ok(Token::Times))));
      assert_eq!(l.next(), Some((1, Ok(Token::DivideFloor))));
      assert_eq!(l.next(), Some((1, Ok(Token::Divide))));
      assert_eq!(l.next(), Some((1, Ok(Token::Mod))));
      assert_eq!(l.next(), Some((1, Ok(Token::At))));
      assert_eq!(l.next(), Some((1, Ok(Token::Lshift))));
      assert_eq!(l.next(), Some((1, Ok(Token::Rshift))));
      assert_eq!(l.next(), Some((1, Ok(Token::BitAnd))));
      assert_eq!(l.next(), Some((1, Ok(Token::BitOr))));
      assert_eq!(l.next(), Some((1, Ok(Token::BitXor))));
      assert_eq!(l.next(), Some((1, Ok(Token::BitNot))));
      assert_eq!(l.next(), Some((1, Ok(Token::LT))));
      assert_eq!(l.next(), Some((1, Ok(Token::GT))));
      assert_eq!(l.next(), Some((1, Ok(Token::LE))));
      assert_eq!(l.next(), Some((1, Ok(Token::GE))));
      assert_eq!(l.next(), Some((1, Ok(Token::EQ))));
      assert_eq!(l.next(), Some((1, Ok(Token::NE))));
      assert_eq!(l.next(), Some((1, Err(LexerError::InvalidSymbol('!')))));
      assert_eq!(l.next(), Some((1, Ok(Token::Ellipsis))));
   }

   #[test]
   fn test_keywords()
   {
      let chars = "false False None True and as assert async await break class continue def del defdel elif else except finally for from \nglobal if import in is lambda nonlocal not or pass raise return try while with yield\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("false".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::False))));
      assert_eq!(l.next(), Some((1, Ok(Token::None))));
      assert_eq!(l.next(), Some((1, Ok(Token::True))));
      assert_eq!(l.next(), Some((1, Ok(Token::And))));
      assert_eq!(l.next(), Some((1, Ok(Token::As))));
      assert_eq!(l.next(), Some((1, Ok(Token::Assert))));
      //assert_eq!(l.next(), Some((1, Ok(Token::Async))));
      //assert_eq!(l.next(), Some((1, Ok(Token::Await))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("async".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("await".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Break))));
      assert_eq!(l.next(), Some((1, Ok(Token::Class))));
      assert_eq!(l.next(), Some((1, Ok(Token::Continue))));
      assert_eq!(l.next(), Some((1, Ok(Token::Def))));
      assert_eq!(l.next(), Some((1, Ok(Token::Del))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("defdel".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Elif))));
      assert_eq!(l.next(), Some((1, Ok(Token::Else))));
      assert_eq!(l.next(), Some((1, Ok(Token::Except))));
      assert_eq!(l.next(), Some((1, Ok(Token::Finally))));
      assert_eq!(l.next(), Some((1, Ok(Token::For))));
      assert_eq!(l.next(), Some((1, Ok(Token::From))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((2, Ok(Token::Global))));
      assert_eq!(l.next(), Some((2, Ok(Token::If))));
      assert_eq!(l.next(), Some((2, Ok(Token::Import))));
      assert_eq!(l.next(), Some((2, Ok(Token::In))));
      assert_eq!(l.next(), Some((2, Ok(Token::Is))));
      assert_eq!(l.next(), Some((2, Ok(Token::Lambda))));
      assert_eq!(l.next(), Some((2, Ok(Token::Nonlocal))));
      assert_eq!(l.next(), Some((2, Ok(Token::Not))));
      assert_eq!(l.next(), Some((2, Ok(Token::Or))));
      assert_eq!(l.next(), Some((2, Ok(Token::Pass))));
      assert_eq!(l.next(), Some((2, Ok(Token::Raise))));
      assert_eq!(l.next(), Some((2, Ok(Token::Return))));
      assert_eq!(l.next(), Some((2, Ok(Token::Try))));
      assert_eq!(l.next(), Some((2, Ok(Token::While))));
      assert_eq!(l.next(), Some((2, Ok(Token::With))));
      assert_eq!(l.next(), Some((2, Ok(Token::Yield))));
      assert_eq!(l.next(), Some((2, Ok(Token::Newline))));
   }

   #[test]
   fn test_strings_1()
   {
      let chars = "'abc 123 \txyz@\")#*)@'\n\"wfe wf w fwe'fwefw\"\n\"abc\n'last line'\n'just\\\n   kidding   \\\n \t kids'\n'xy\\\n  zq\nxyz'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("abc 123 \txyz@\")#*)@".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((2, Ok(Token::String("wfe wf w fwe'fwefw".to_owned())))));
      assert_eq!(l.next(), Some((2, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Err(LexerError::UnterminatedString))));
      assert_eq!(l.next(), Some((3, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((4, Ok(Token::String("last line".to_owned())))));
      assert_eq!(l.next(), Some((4, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((5, Ok(Token::String("just   kidding    \t kids".to_owned())))));
      assert_eq!(l.next(), Some((7, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((9, Err(LexerError::UnterminatedString))));
   }

   #[test]
   fn test_strings_2()
   {
      let chars = "'abc' \"def\" \\\n'123'\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("abcdef123".to_owned())))));
      assert_eq!(l.next(), Some((2, Ok(Token::Newline))));
   }

   #[test]
   fn test_strings_3()
   {
      let chars = "''' abc ' '' '''\n\"\"\"xyz\"\"\"\n'''abc\n \tdef\n123'''\n'''abc\\\n \tdef\\\n123'''\n'''abc\ndef";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String(" abc ' '' ".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((2, Ok(Token::String("xyz".to_owned())))));
      assert_eq!(l.next(), Some((2, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Ok(Token::String("abc\n \tdef\n123".to_owned())))));
      assert_eq!(l.next(), Some((5, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((6, Ok(Token::String("abc \tdef123".to_owned())))));
      assert_eq!(l.next(), Some((8, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((11, Err(LexerError::UnterminatedTripleString))));
   }

   #[test]
   fn test_strings_4()
   {
      let chars = "'\\\\'\n'\\''\n'\\\"'\n'\\a'\n'\\b'\n'\\f'\n'\\n'\n'\\r'\n'\\t'\n'\\v'\n'\\m'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("\\".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((2, Ok(Token::String("'".to_owned())))));
      assert_eq!(l.next(), Some((2, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Ok(Token::String("\"".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((4, Ok(Token::String("\x07".to_owned())))));
      assert_eq!(l.next(), Some((4, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((5, Ok(Token::String("\x08".to_owned())))));
      assert_eq!(l.next(), Some((5, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((6, Ok(Token::String("\x0C".to_owned())))));
      assert_eq!(l.next(), Some((6, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((7, Ok(Token::String("\n".to_owned())))));
      assert_eq!(l.next(), Some((7, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((8, Ok(Token::String("\r".to_owned())))));
      assert_eq!(l.next(), Some((8, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((9, Ok(Token::String("\t".to_owned())))));
      assert_eq!(l.next(), Some((9, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((10, Ok(Token::String("\x0B".to_owned())))));
      assert_eq!(l.next(), Some((10, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((11, Ok(Token::String("\\m".to_owned())))));
   }

   #[test]
   fn test_strings_5()
   {
      let chars = "'\\007'\n'\\7'\n'\\175'\n'\\x07'\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("\x07".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((2, Ok(Token::String("\x07".to_owned())))));
      assert_eq!(l.next(), Some((2, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Ok(Token::String("}".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((4, Ok(Token::String("\x07".to_owned())))));
      assert_eq!(l.next(), Some((4, Ok(Token::Newline))));
   }

   #[test]
   fn test_strings_6()
   {
      let chars = "'\\x'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Err(LexerError::HexEscapeShort))));
   }

   #[test]
   fn test_strings_7()
   {
      let chars = "'\\x7'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Err(LexerError::HexEscapeShort))));
   }

   #[test]
   fn test_strings_8()
   {
      let chars = "'\\N{monkey}'\n'\\N{BLACK STAR}'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("🐒".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((2, Ok(Token::String("★".to_owned())))));
   }

   #[test]
   fn test_strings_9()
   {
      let chars = "'\\N{monkey'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Err(LexerError::MalformedNamedUnicodeEscape))));
   }

   #[test]
   fn test_strings_10()
   {
      let chars = "'\\Nmonkey'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Err(LexerError::MalformedNamedUnicodeEscape))));
   }

   #[test]
   fn test_strings_11()
   {
      let chars = "'\\N{fhefaefi}'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Err(LexerError::UnknownUnicodeName("fhefaefi".to_owned())))));
   }

   #[test]
   fn test_strings_12()
   {
      let chars = "'\\u262f'\n'\\U00002D5E'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("☯".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((2, Ok(Token::String("ⵞ".to_owned())))));
   }

   #[test]
   fn test_strings_13()
   {
      let chars = "'\\u262'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Err(LexerError::MalformedUnicodeEscape))));
   }

   #[test]
   fn test_strings_14()
   {
      let chars = "'\\U00002D'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Err(LexerError::MalformedUnicodeEscape))));
   }

   #[test]
   fn test_strings_15()
   {
      let chars = "'\\u262f262f'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("☯262f".to_owned())))));
   }

   #[test]
   fn test_strings_16()
   {
      let chars = "unlikely u'abc' u '123' U\"\"\"def\"\"\" u\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("unlikely".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::String("abc".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("u".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::String("123def".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("u".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
   }

   #[test]
   fn test_strings_17()
   {
      let chars = "r'\\txyz \\\n \\'fefe \\N{monkey}'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("\\txyz \\\n \\'fefe \\N{monkey}".to_owned())))));
   }

   #[test]
   fn test_strings_18()
   {
      let chars = "r'''\\txyz \\\n \\'fefe \\N{monkey}''''hello\\040\\700\\300'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("\\txyz \\\n \\'fefe \\N{monkey}hello ǀÀ".to_owned())))));
   }

   #[test]
   fn test_strings_19()
   {
      let chars = "'''hello\\\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((2, Err(LexerError::UnterminatedTripleString))));
   }

   #[test]
   fn test_byte_strings_1()
   {
      let chars = "b'''hello'''";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![104, 101, 108, 108, 111])))));
   }

   #[test]
   fn test_byte_strings_2()
   {
      let chars = "b'''hello\nblah'''";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![104, 101, 108, 108, 111, 10, 98, 108, 97, 104])))));
   }

   #[test]
   fn test_byte_strings_3()
   {
      let chars = "b'\\x26\\040'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![38, 32])))));
   }

   #[test]
   fn test_byte_strings_4()
   {
      let chars = "b'\\x26\\040\\700\\300'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![38, 32, 192, 192])))));
   }

   #[test]
   fn test_byte_strings_5()
   {
      let chars = "b'abc\\\n  \t 123'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![97, 98, 99, 32, 32, 9, 32, 49, 50, 51])))));
   }

   #[test]
   fn test_byte_strings_6()
   {
      let chars = "b'abc\\\n  \t 123' \\\n  b'123'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![97, 98, 99, 32, 32, 9, 32, 49, 50, 51, 49, 50, 51])))));
   }

   #[test]
   fn test_byte_strings_7()
   {
      let chars = "rb'abc\\' \\\n  \t 123'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![97, 98, 99, 92, 39, 32, 92, 10, 32, 32, 9, 32, 49, 50, 51])))));
   }

   #[test]
   fn test_byte_strings_8()
   {
      let chars = "Br'abc\\' \\\n  \t' bR' 123'";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Bytes(vec![97, 98, 99, 92, 39, 32, 92, 10, 32, 32, 9, 32, 49, 50, 51])))));
   }

   #[test]
   fn test_implicit_1()
   {
      let chars = "(1 + \n      2 \n)";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Lparen))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("1".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Plus))));
      assert_eq!(l.next(), Some((2, Ok(Token::DecInteger("2".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Rparen))));
   }

   #[test]
   fn test_implicit_2()
   {
      let chars = "   (1 + \n   (   2 \n + 9 \n ) * \n      2 \n )\n2";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((1, Ok(Token::Lparen))));
      assert_eq!(l.next(), Some((1, Ok(Token::DecInteger("1".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Plus))));
      assert_eq!(l.next(), Some((2, Ok(Token::Lparen))));
      assert_eq!(l.next(), Some((2, Ok(Token::DecInteger("2".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Plus))));
      assert_eq!(l.next(), Some((3, Ok(Token::DecInteger("9".to_owned())))));
      assert_eq!(l.next(), Some((4, Ok(Token::Rparen))));
      assert_eq!(l.next(), Some((4, Ok(Token::Times))));
      assert_eq!(l.next(), Some((5, Ok(Token::DecInteger("2".to_owned())))));
      assert_eq!(l.next(), Some((6, Ok(Token::Rparen))));
      assert_eq!(l.next(), Some((6, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((7, Ok(Token::Dedent))));
      assert_eq!(l.next(), Some((7, Ok(Token::DecInteger("2".to_owned())))));
   }

   #[test]
   fn test_implicit_3()
   {
      let chars = "('abc' \n      'def' \n)";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Lparen))));
      assert_eq!(l.next(), Some((1, Ok(Token::String("abcdef".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Rparen))));
   }

   #[test]
   fn test_implicit_4()
   {
      let chars = "def abc(a, g,\n         c):\n   first";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Def))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("abc".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Lparen))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("a".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Comma))));
      assert_eq!(l.next(), Some((1, Ok(Token::Identifier("g".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Comma))));
      assert_eq!(l.next(), Some((2, Ok(Token::Identifier("c".to_owned())))));
      assert_eq!(l.next(), Some((2, Ok(Token::Rparen))));
      assert_eq!(l.next(), Some((2, Ok(Token::Colon))));
      assert_eq!(l.next(), Some((2, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Ok(Token::Indent))));
      assert_eq!(l.next(), Some((3, Ok(Token::Identifier("first".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((0, Ok(Token::Dedent))));
   }

   #[test]
   fn test_implicit_5()
   {
      let chars = "('abc'\n   #  'def' \n)";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::Lparen))));
      assert_eq!(l.next(), Some((1, Ok(Token::String("abc".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Rparen))));
   }

   #[test]
   fn test_implicit_6()
   {
      let chars = "'abc'\n   #  'def' \n123\n";
      let mut l = Lexer::new(chars.lines());
      assert_eq!(l.next(), Some((1, Ok(Token::String("abc".to_owned())))));
      assert_eq!(l.next(), Some((1, Ok(Token::Newline))));
      assert_eq!(l.next(), Some((3, Ok(Token::DecInteger("123".to_owned())))));
      assert_eq!(l.next(), Some((3, Ok(Token::Newline))));
      assert_eq!(l.next(), None);
   }
}
