use std::iter::Peekable;

pub struct Lexer<'a>
{
   indent_stack: Vec<u32>,
   line_number: u32,
   char_number: u32,
   chars: Peekable<&'a mut Iterator<Item=char>>
}

impl <'a> Lexer<'a>
{
   fn new(chars: &'a mut Iterator<Item=char>) -> Self
   {
      Lexer{indent_stack: vec![],
         line_number: 1,
         char_number: 1,
         chars: chars.peekable()}
   }

   fn next_token(&mut self) -> Option<(u32, String)>
   {
      match self.chars.peek()
      {
         Some(&c) if Lexer::is_xid_start(c) =>
            {
               let result = self.build_identifier();
               self.consume_space_to_next();
               Some(result)
            },
         Some(&c) if Lexer::is_newline(c) => Some(self.build_newline()),
         _ => None
      }
   }

   fn build_identifier(&mut self) -> (u32, String)
   {
      let mut token = String::new();
      token.push(self.chars.next().unwrap());

      self.char_number += 1;

      while self.chars.peek().is_some() &&
         Lexer::is_xid_continue(*self.chars.peek().unwrap())
      {
         token.push(self.chars.next().unwrap());
         self.char_number += 1;
      }

      (self.line_number, token)
   }

   fn build_newline(&mut self) -> (u32, String)
   {
      let c = self.chars.next().unwrap();

      if c == '\r' && self.chars.peek().is_some() &&
         *self.chars.peek().unwrap() == '\n'
      {
         self.chars.next();
      }

      let line_number = self.line_number;
      self.line_number += 1;
      self.char_number = 0;

      (line_number, "newline".to_string())
   }

   fn consume_space_to_next(&mut self)
   {
      while self.chars.peek().is_some() &&
         Lexer::is_space_between(*self.chars.peek().unwrap())
      {
         self.chars.next();
         self.char_number += 1;
      }
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

   fn is_space_between(c: char) -> bool
   {
      c == ' ' || c == '\t' || c == '\x0C'
   }

   fn is_newline(c: char) -> bool
   {
      c == '\r' || c == '\n'
   }
}

impl <'a> Iterator for Lexer<'a>
{
   type Item = (u32, String);

   fn next(&mut self) -> Option<Self::Item>
   {
      match self.chars.peek()
      {
         None => None,
         Some(_) => self.next_token()
      }
   }
}

#[cfg(test)]
mod tests
{
   use super::Lexer;

   #[test]
   fn test_creation()
   {
      let chars: &mut Iterator<Item=char> = &mut "abcdef 123".chars();
      let l = Lexer::new(chars);
      assert_eq!(l.chars.collect::<Vec<char>>(),
         vec!['a', 'b', 'c', 'd', 'e', 'f', ' ', '1', '2', '3']);
   }   

   #[test]
   fn test_identifiers()
   {
      let chars = &mut "abf  \x0C xyz 	e2f  \rmq3\n12\r\n3\r23\n".chars() as
         &mut Iterator<Item=char>;
      let mut l = Lexer::new(chars);
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "abf")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "xyz")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "e2f")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((1, "newline")));
      assert_eq!(l.next().as_ref().map(|p| (p.0, &p.1[..])), Some((2, "mq3")));
   }   

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
}
