use std::iter::Iterator;

pub struct DoublePeekable<I>
   where I: Iterator
{
   first: Option<I::Item>,
   second: Option<I::Item>,
   iter: I,
}

impl <I> DoublePeekable<I>
   where I: Iterator
{
   pub fn new(mut iter: I)
      -> Self
   {
      let first = iter.next();
      let second = iter.next();
      DoublePeekable{iter: iter, first: first, second: second}
   }

   pub fn peek(&self)
      -> Option<&I::Item>
   {
      self.first.as_ref()
   }

   pub fn peek_second(&self)
      -> Option<&I::Item>
   {
      self.second.as_ref()
   }

   fn get_next(&mut self)
      -> Option<I::Item>
   {
      let result = self.first.take();

      self.first = self.second.take();
      if self.first.is_some()
      {
         // technically, an interator is not required to return None
         // if next is called again after None has been returned
         self.second = self.iter.next();
      }
      result
   }
}

impl <I> Iterator for DoublePeekable<I>
   where I: Iterator
{
   type Item = I::Item;

   fn next(&mut self)
      -> Option<I::Item>
   {
      self.get_next()
   }
}

#[cfg(test)]
mod test
{
   use super::DoublePeekable;
   #[test]
   fn test_peek()
   {
      let mut iter = DoublePeekable::new(1..6);
      assert_eq!(2, *iter.peek_second().unwrap());
      assert_eq!(1, *iter.peek().unwrap());
      assert_eq!(1, *iter.peek().unwrap());
      assert_eq!(2, *iter.peek_second().unwrap());
      assert_eq!(2, *iter.peek_second().unwrap());
      assert_eq!(1, *iter.peek().unwrap());
      assert_eq!(1, iter.next().unwrap());

      assert_eq!(3, *iter.peek_second().unwrap());
      assert_eq!(2, *iter.peek().unwrap());
      assert_eq!(2, *iter.peek().unwrap());
      assert_eq!(3, *iter.peek_second().unwrap());
      assert_eq!(3, *iter.peek_second().unwrap());
      assert_eq!(2, *iter.peek().unwrap());
      assert_eq!(2, iter.next().unwrap());

      assert_eq!(4, *iter.peek_second().unwrap());
      assert_eq!(3, *iter.peek().unwrap());
      assert_eq!(3, *iter.peek().unwrap());
      assert_eq!(4, *iter.peek_second().unwrap());
      assert_eq!(4, *iter.peek_second().unwrap());
      assert_eq!(3, *iter.peek().unwrap());
      assert_eq!(3, iter.next().unwrap());

      assert_eq!(5, *iter.peek_second().unwrap());
      assert_eq!(4, *iter.peek().unwrap());
      assert_eq!(4, *iter.peek().unwrap());
      assert_eq!(5, *iter.peek_second().unwrap());
      assert_eq!(5, *iter.peek_second().unwrap());
      assert_eq!(4, *iter.peek().unwrap());
      assert_eq!(4, iter.next().unwrap());

      assert_eq!(None, iter.peek_second());
      assert_eq!(5, *iter.peek().unwrap());
      assert_eq!(5, *iter.peek().unwrap());
      assert_eq!(None, iter.peek_second());
      assert_eq!(None, iter.peek_second());
      assert_eq!(5, *iter.peek().unwrap());
      assert_eq!(5, iter.next().unwrap());

      assert_eq!(None, iter.peek_second());
      assert_eq!(None, iter.peek());
      assert_eq!(None, iter.peek());
      assert_eq!(None, iter.peek_second());
      assert_eq!(None, iter.peek_second());
      assert_eq!(None, iter.peek());
      assert_eq!(None, iter.next());

      assert_eq!(None, iter.peek_second());
      assert_eq!(None, iter.peek());
      assert_eq!(None, iter.peek());
      assert_eq!(None, iter.peek_second());
      assert_eq!(None, iter.peek_second());
      assert_eq!(None, iter.peek());
      assert_eq!(None, iter.next());
   }
}
