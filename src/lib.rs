use std::str::Chars;

pub struct Lexer<'a>
{
   peek: Option<char>,
   indent_stack: Vec<u32>,
   line_number: u32,
   chars: Chars<'a>
}

impl <'a> Lexer<'a>
{
   fn new(str: &'a str) -> Self
   {
      let mut chars = str.chars();
      Lexer{peek: chars.next(),
         indent_stack: vec![],
         line_number: 1,
         chars: chars}
   }

   fn next_token(&mut self) -> Option<(u32, String)>
   {
      if Lexer::is_xid_start(self.peek.unwrap())
      {
         Some(self.build_identifier())
      }
      else
      {
         None
      }
   }

   fn build_identifier(&mut self) -> (u32, String)
   {
      let mut token = String::new();
      token.push(self.peek.unwrap());
      self.peek = self.chars.next();

      while self.peek.is_some() && Lexer::is_xid_continue(self.peek.unwrap())
      {
         token.push(self.peek.unwrap());
         self.peek = self.chars.next();
      }

      (0, token)
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
}

impl <'a> Iterator for Lexer<'a>
{
   type Item = (u32, String);

   fn next(&mut self) -> Option<Self::Item>
   {
      match self.peek
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
      let l = Lexer::new("abcdef 123");
      assert_eq!(l.chars.collect::<Vec<char>>(),
         vec!['b', 'c', 'd', 'e', 'f', ' ', '1', '2', '3']);
   }   

   #[test]
   fn test_identifiers()
   {
      let mut l = Lexer::new("abf\n12\r\n3\r23\n").map(|(_,i)| i);
      assert_eq!(l.next().as_ref().map(|s| &s[..]), Some("abf"));
   }   

   #[test]
   fn test_line_numbers()
   {
      let mut l = Lexer::new("abf\n12\r\n3\r23\n").map(|(n,_)| n);
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
