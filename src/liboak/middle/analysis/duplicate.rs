// Copyright 2014 Pierre Talbot (IRCAM)

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub use std::collections::HashMap;

use middle::analysis::ast::*;
use partial::Partial::*;

pub fn rule_duplicate(mut grammar: AGrammar, rules: Vec<Rule>) -> Partial<AGrammar>
{
  DuplicateItem::analyse(&grammar, rules.into_iter(), String::from("rule"))
  .map(move |rules| {
    grammar.rules = rules.into_iter().map(|x| x.1).collect();
    grammar
  })
}

pub fn rust_functions_duplicate(mut grammar: AGrammar, items: Vec<syn::Item>) -> Partial<AGrammar>
{
  let mut functions = vec![];
  let mut others = vec![];
  for item in items {
    if let syn::Item::Fn(fun) = item {
      functions.push(fun);
    }
    else {
      others.push(item);
    }
  }
  DuplicateItem::analyse(&grammar, functions.into_iter(), String::from("rust function"))
    .map(move |functions| {
      grammar.rust_functions = functions.into_iter().collect();
      grammar.rust_items = others;
      grammar
    })
}

struct DuplicateItem<'a, Item>
{
  grammar: &'a AGrammar,
  items: Vec<(Ident, Item)>,
  has_duplicate: bool,
  what_is_duplicated: String
}

impl<'a, Item> DuplicateItem<'a, Item> where
 Item: ItemIdent + Spanned
{
  pub fn analyse<ItemIter>(grammar: &'a AGrammar, iter: ItemIter, item_kind: String)
    -> Partial<Vec<(Ident, Item)>> where
   ItemIter: Iterator<Item=Item>
  {
    DuplicateItem {
      grammar: grammar,
      items: vec![],
      has_duplicate: false,
      what_is_duplicated: item_kind
    }.populate(iter)
     .make()
  }

  fn populate<ItemIter: Iterator<Item=Item>>(mut self, iter: ItemIter)
    -> DuplicateItem<'a, Item>
  {
    for item in iter {
      let ident = item.ident();
      if self.items.iter().any(|&(ref id,_)| *id == ident) {
        let &(_, ref dup_item) = self.items.iter().find(|&&(ref id,_)| *id == ident).unwrap();
        self.duplicate_items(dup_item, item);
        self.has_duplicate = true;
      }
      else {
        self.items.push((ident, item));
      }
    }
    self
  }

  fn duplicate_items(&self, pre: &Item, current: Item) {
    current.span().unstable()
      .error(format!("duplicate definition of {} with name `{}`", self.what_is_duplicated, current.ident()))
      .span_note(pre.span().unstable(), format!("previous definition of `{}` here", pre.ident()))
      .emit();
  }

  fn make(self) -> Partial<Vec<(Ident, Item)>> {
    if self.has_duplicate {
      Fake(self.items)
    } else {
      Value(self.items)
    }
  }
}
