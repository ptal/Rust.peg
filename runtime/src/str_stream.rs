// Copyright 2015 Pierre Talbot (IRCAM)

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Implementation of `Stream` for `&'a str` type. It implements all traits required by `CharStream`.

use stream::*;
use std::cmp::{Ordering, min};
use super::*;
pub use std::ops::Range;
pub use syntex_pos::Span;

impl<'a> Stream for &'a str
{
  type Output = StrStream<'a>;
  fn stream(self) -> StrStream<'a> {
    StrStream::new(self)
  }
}

impl<'a> Stream for &'a String
{
  type Output = StrStream<'a>;
  fn stream(self) -> StrStream<'a> {
    self.as_str().stream()
  }
}

/// Represents a stream from a `&'a str`. It implements all traits required by `CharStream`.
#[derive(Clone, Hash, Debug)]
pub struct StrStream<'a>
{
  raw_data: &'a str,
  bytes_offset: usize
}

impl<'a> StrStream<'a>
{
  fn new(raw_data: &'a str) -> StrStream<'a> {
    StrStream {
      raw_data: raw_data,
      bytes_offset: 0
    }
  }

  #[inline(always)]
  fn assert_same_raw_data(&self, other: &StrStream<'a>) {
    debug_assert!(self.raw_data.as_ptr() == other.raw_data.as_ptr(),
      "Operations between two streams are only defined when they share the same raw data.");
  }

  // Partially taken from https://github.com/kevinmehall/rust-peg/blob/master/src/translate.rs
  pub fn line_column(&self) -> (usize, usize) {
    let mut remaining = self.bytes_offset;
    let mut line_no = 1usize;
    for line in self.raw_data.lines() {
      let line_len = line.len() + 1;
      if remaining < line_len {
        break;
      }
      remaining -= line_len;
      line_no += 1;
    }
    (line_no, remaining + 1)
  }

  pub fn bytes_offset(&self) -> usize {
    self.bytes_offset
  }

  pub fn current_char(&self) -> Option<char> {
    self.raw_data[self.bytes_offset..].chars().next()
  }
}

impl<'a> Iterator for StrStream<'a>
{
  type Item = char;
  fn next(&mut self) -> Option<Self::Item> {
    if self.bytes_offset < self.raw_data.len() {
      let current = self.current_char().unwrap();
      self.bytes_offset += current.len_utf8();
      Some(current)
    } else {
      None
    }
  }
}

impl<'a> PartialEq for StrStream<'a>
{
  fn eq(&self, other: &Self) -> bool {
    self.assert_same_raw_data(other);
    self.bytes_offset == other.bytes_offset
  }
}

impl<'a> Eq for StrStream<'a> {}

impl<'a> PartialOrd for StrStream<'a>
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.assert_same_raw_data(other);
    self.bytes_offset.partial_cmp(&other.bytes_offset)
  }
}

impl<'a> Ord for StrStream<'a>
{
  fn cmp(&self, other: &Self) -> Ordering {
    self.assert_same_raw_data(other);
    self.bytes_offset.cmp(&other.bytes_offset)
  }
}

impl<'a> Location for StrStream<'a>
{
  fn location(&self) -> String {
    let (line, column) = self.line_column();
    format!("{}:{}", line, column)
  }
}

impl<'a> CodeSnippet for StrStream<'a>
{
  fn code_snippet(&self, len_hint: usize) -> String {
    let total_len = self.raw_data.len();
    let current_offset = self.bytes_offset;
    if current_offset == total_len {
      String::from("<end-of-file>")
    }
    else {
      let len = min(total_len - current_offset, len_hint);
      String::from(&self.raw_data[current_offset..][..len])
    }
  }
}

impl<'a> ConsumePrefix<&'static str> for StrStream<'a>
{
  fn consume_prefix(&mut self, prefix: &'static str) -> bool {
    let current_offset = self.bytes_offset;
    let end_offset = current_offset + prefix.len();
    if end_offset <= self.raw_data.len()
     && &self.raw_data.as_bytes()[current_offset..end_offset] == prefix.as_bytes()
    {
      self.bytes_offset = end_offset;
      true
    } else {
      false
    }
  }
}

impl<'a> HasNext for StrStream<'a>
{
  fn has_next(&self) -> bool {
    self.bytes_offset < self.raw_data.len()
  }
}

impl<'a> StreamSpan for Range<StrStream<'a>>
{
  type Output = Span;
  fn stream_span(&self) -> Self::Output {
    make_span(
      self.start.bytes_offset,
      self.end.bytes_offset)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn consume_prefix_test<'a>(stream: &StrStream<'a>, prefix: &'static str,
    prefix_match: bool, next_char: Option<char>)
  {
    let mut s2 = stream.clone();
    assert_eq!(s2.consume_prefix(prefix), prefix_match);
    assert!(s2.next() == next_char);
  }

  #[test]
  fn test_consume_prefix() {
    let s1 = &"abc".stream();
    consume_prefix_test(s1, "abc", true, None);
    consume_prefix_test(s1, "ab", true, Some('c'));
    consume_prefix_test(s1, "", true, Some('a'));
    consume_prefix_test(s1, "ac", false, Some('a'));
    consume_prefix_test(s1, "z", false, Some('a'));
  }

  fn test_str_stream<'a, I>(mut s1: StrStream<'a>, chars: I) where
   I: Iterator<Item=char>
  {
    let s1_init = s1.clone();
    let mut s2 = s1_init.clone();
    for c in chars {
      assert!(s1 == s2);
      assert_eq!(s1.next().unwrap(), c);
      assert!(s1 > s1_init);
      assert!(s1 > s2);
      s2 = s1.clone();
    }
    assert_eq!(s1.next(), None);
    assert_eq!(s2.next(), None);
    assert!(s1 > s1_init);
    assert!(s1 == s2);
  }

  #[test]
  fn test_stream() {
    let abc = "abc";
    test_str_stream(abc.stream(), abc.chars());
  }

  #[test]
  fn test_string_stream() {
    let abc = String::from("abc");
    test_str_stream(abc.stream(), abc.chars());
  }

  #[test]
  fn test_empty_stream() {
    let mut empty = "".stream();
    assert_eq!(empty.bytes_offset, 0);
    assert_eq!(empty.next(), None);
    assert_eq!(empty.next(), None);
    assert_eq!(empty.bytes_offset, 0);
    assert!(empty == empty);
    assert!(!(empty > empty));
    let empty2 = empty.clone();
    assert!(empty == empty2);
    assert!(!(empty > empty2));
  }

  fn test_unrelated_streams<R, F>(op: F) where
   F: FnOnce(&StrStream<'static>, &StrStream<'static>) -> R
  {
    let s1 = "abc".stream();
    let s2 = "def".stream();
    op(&s1, &s2);
  }

  #[test]
  #[should_panic]
  fn unrelated_stream_eq() {
    test_unrelated_streams(|a, b| a == b);
  }

  #[test]
  #[should_panic]
  fn unrelated_stream_partial_ord() {
    test_unrelated_streams(|a, b| a.partial_cmp(b));
  }

  #[test]
  #[should_panic]
  fn unrelated_stream_ord() {
    test_unrelated_streams(|a, b| a.cmp(b));
  }
}
