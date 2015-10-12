extern crate unicode_segmentation;
use unicode_segmentation::{UnicodeSegmentation, Graphemes};

pub struct Lexer<'a>
{
   chars: Graphemes<'a>
}

impl <'a> Lexer<'a>
{
   fn new(str: &'a str) -> Self
   {
      Lexer{chars: UnicodeSegmentation::graphemes(str, true)}
   }
}

#[cfg(test)]
mod tests
{
   use super::Lexer;

   #[test]
   fn test_creation()
   {
      println!("monkey");
      let l = Lexer::new("abcdef 123");
      assert_eq!(l.chars.collect::<Vec<&str>>(),
         vec!["a", "b", "c", "d", "e", "f", " ", "1", "2", "3"]);
   }   
}
