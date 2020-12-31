// Copyright 2016 Pierre Talbot (IRCAM)

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use middle::analysis::ast::*;
use std::mem::swap;
use std::collections::{HashMap, HashSet};

/// Well-formedness attributes, it represents the possible behavior of an expression.

#[derive(Clone, Copy, PartialEq, Eq)]
struct WFA
{
  can_fail: bool,
  can_succeed: bool,
  always_consume: bool,
  never_consume: bool,
}

impl WFA
{
  fn always_succeed(never_consume: bool) -> Self {
    WFA {
      can_fail: false,
      can_succeed: true,
      always_consume: false,
      never_consume
    }
  }
}

impl Default for WFA {
  fn default() -> Self {
    WFA {
      can_fail: true,
      can_succeed: true,
      always_consume: true,
      never_consume: false
    }
  }
}

pub struct WellFormedness<'a>
{
  grammar: &'a AGrammar,
  recursion_path: Vec<(Ident, bool)>,
  consumed_input: bool,
  rules_wfa: HashMap<Ident, WFA>,
  reached_fixpoint: bool,
  well_formed: bool,
  errors: HashSet<usize> // Whether we already spot an error on this rule (to avoid multi-reporting).
}

// Start with an empty set of the expression attributes.

impl<'a> WellFormedness<'a>
{
  pub fn analyse(grammar: AGrammar) -> Partial<AGrammar> {
    if WellFormedness::is_well_formed(&grammar) {
      Partial::Value(grammar)
    } else {
      Partial::Nothing
    }
  }

  fn is_well_formed(grammar: &'a AGrammar) -> bool {
    let mut analyser = WellFormedness::new(grammar);
    analyser.visit_rules();
    analyser.well_formed
  }

  fn new(grammar: &'a AGrammar) -> Self {
    WellFormedness {
      grammar: grammar,
      recursion_path: vec![],
      consumed_input: false,
      rules_wfa: grammar.rules.iter()
        .map(|rule| (rule.ident(), WFA::default()))
        .collect(),
      reached_fixpoint: false,
      well_formed: true,
      errors: HashSet::new()
    }
  }

  fn visit_rules(&mut self) {
    while !self.reached_fixpoint && self.well_formed {
      self.reached_fixpoint = true;
      for rule in self.grammar.rules.iter() {
        self.visit_rule(&rule.ident());
        if !self.well_formed { break; }
      }
    }
  }

  fn visit_rule(&mut self, rule: &Ident) -> WFA {
    if self.is_rec(rule) {
      if !self.consume_input_since(rule) && !self.consumed_input {
        self.error_left_recursion(rule);
      }
    }
    else {
      self.push_rule_in_path(rule);
      let wfa = self.visit_rule_expr(rule);
      self.pop_rule_in_path();
      self.fixpoint_update(wfa, rule);
    }
    self.rules_wfa[&rule]
  }

  fn visit_rule_expr(&mut self, rule: &Ident) -> WFA {
    let expr_idx = self.grammar.expr_index_of_rule(rule);
    self.visit_expr(expr_idx)
  }

  fn push_rule_in_path(&mut self, rule: &Ident) {
    self.recursion_path.push((rule.clone(), self.consumed_input));
    self.consumed_input = false;
  }

  fn pop_rule_in_path(&mut self) {
    let (_, old_consumed_input) = self.recursion_path.pop().unwrap();
    self.consumed_input = old_consumed_input;
  }

  fn fixpoint_update(&mut self, wfa: WFA, rule: &Ident) {
    if wfa != self.rules_wfa[&rule] {
      self.reached_fixpoint = false;
      *self.rules_wfa.get_mut(&rule).unwrap() = wfa;
    }
  }

  fn is_rec(&self, rule: &Ident) -> bool {
    self.recursion_path.iter().any(|&(ref r,_)| r == rule)
  }

  fn rec_path_from(&self, rule: &Ident) -> Vec<(Ident, bool)> {
    self.recursion_path.iter().cloned()
      .rev()
      .take_while(|&(ref r, _)| r != rule)
      .collect()
  }

  fn consume_input_since(&self, rule: &Ident) -> bool {
    let mut has_consumed = false;
    for (_, consumed_input) in self.rec_path_from(rule) {
      has_consumed |= consumed_input;
    }
    has_consumed
  }

  fn save(&self) -> bool {
    self.consumed_input
  }

  fn restore(&mut self, savepoint: bool) {
    self.consumed_input = savepoint;
  }

  fn error_left_recursion(&mut self, rule_id: &Ident) {
    self.well_formed = false;
    let rule = self.grammar.find_rule_by_ident(rule_id);
    if self.register_error(rule.expr_idx) {
      let mut rec_path: Vec<_> = vec![rule_id.clone()];
      rec_path.extend(self.rec_path_from(rule_id).into_iter()
        .map(|(r,_)| r)
        .rev());
      rule.span().unstable().error(format!(
        "Left-recursion is not supported in Oak; the following rule cycle \
        do not consume any input and would therefore loop forever\n\
        Detected cycle: {}\n\
        Solution: Rewrite one of the incriminated rules such that it \
        consumes at least one atom in the input before calling \
        the next one. Usually, left-recursion is rewritten with a \
        repeat operator (`e*` or `e+`).",
        display_path_cycle(&rec_path))).emit();
    }
  }

  fn error_never_succeed(&mut self, expr_idx: usize) {
    if self.register_error(expr_idx) {
      self.well_formed = false;
      self.grammar[expr_idx].span().unstable().error(format!(
        "Expression will never succeed.\n\
        Solution: Remove this expression.")).emit();
    }
  }

  fn error_always_succeed_without_consuming(&mut self, expr_idx: usize) {
    if self.register_error(expr_idx) {
      self.well_formed = false;
      self.grammar[expr_idx].span().unstable().error(format!(
        "Expression will always succeed without consuming any input.\n\
        Solution: Remove this expression.")).emit();
    }
  }

  fn error_loop_repeat(&mut self, expr_idx: usize) {
    if self.register_error(expr_idx) {
      self.well_formed = false;
      self.grammar[expr_idx].span().unstable().error(format!(
        "Infinite loop detected. A repeat operator (`e*` or `e+`) will \
        never stop because the sub-expression does not consume input.\n\
        Solution: Rewrite the expression such that it consumes at least \
        one atom in the input or get rid of the repeat operator.")).emit();
    }
  }

  fn error_unreachable_branches(&mut self, choice: usize, always_succeed_branch: usize)
  {
    if self.register_error(always_succeed_branch) {
      self.well_formed = false;
      self.grammar[choice].span().unstable().error(format!(
        "Unreachable branch in a choice expression. We detected that \
        some branches cannot be reached in this expression.\n\
        Solution: Either remove (or rewrite) this branch or move it \
        to the end of the choice expression."))
      .span_note(self.grammar[always_succeed_branch].span().unstable(), format!(
        "Branch always succeeding"))
      .emit();
    }
  }

  fn register_error(&mut self, expr_idx: usize) -> bool {
    self.errors.insert(expr_idx)
  }
}

impl<'a> ExprByIndex for WellFormedness<'a>
{
  fn expr_by_index(&self, index: usize) -> Expression {
    self.grammar.expr_by_index(index).clone()
  }
}

impl<'a> Visitor<WFA> for WellFormedness<'a>
{
  fn visit_expr(&mut self, this: usize) -> WFA {
    let mut wfa = walk_expr(self, this);
    assert!(wfa.can_fail || wfa.can_succeed,
      "Expression must either fails or succeeds.");
    assert!(!wfa.always_consume || !wfa.never_consume,
      "Expression cannot always and never consume at the same time.");
    if wfa.can_fail && !wfa.can_succeed {
      self.error_never_succeed(this);
      wfa.can_succeed = true; // Error-recovery.
    }
    else if wfa.can_succeed && !wfa.can_fail && wfa.never_consume {
      self.error_always_succeed_without_consuming(this);
      wfa.never_consume = false; // Error-recovery.
    }
    wfa
  }

  fn visit_str_literal(&mut self, _this: usize, literal: String) -> WFA {
    let mut wfa = WFA::default();
    if literal.is_empty() {
      wfa.can_fail = false;
      wfa.always_consume = false;
      // NOTE: the empty string is special, and it can be desirable that it always succeeds without consuming, for some patterns in combination with choice.
      // Therefore, we do not set `never_consume = true`.
    }
    wfa
  }

  fn visit_non_terminal_symbol(&mut self, _this: usize, rule: &Ident) -> WFA {
    self.visit_rule(rule)
  }

  fn visit_repeat(&mut self, this: usize, child: usize) -> WFA {
    let child_wfa = self.visit_expr(child);
    if child_wfa.can_succeed && !child_wfa.always_consume {
      self.error_loop_repeat(this);
      WFA::default()
    }
    else {
      child_wfa
    }
  }

  fn visit_zero_or_more(&mut self, this: usize, child: usize) -> WFA {
    let child_wfa = self.visit_repeat(this, child);
    WFA::always_succeed(child_wfa.never_consume)
  }

  fn visit_optional(&mut self, _this: usize, child: usize) -> WFA {
    let child_wfa = self.visit_expr(child);
    WFA::always_succeed(child_wfa.never_consume)
  }

  fn visit_syntactic_predicate(&mut self, _this: usize, child: usize) -> WFA {
    let child_wfa = self.visit_expr(child);
    let mut wfa = child_wfa;
    wfa.always_consume = false;
    wfa.never_consume = true;
    wfa
  }

  fn visit_not_predicate(&mut self, this: usize, child: usize) -> WFA {
    let mut wfa = self.visit_syntactic_predicate(this, child);
    swap(&mut wfa.can_succeed, &mut wfa.can_fail);
    wfa
  }

  fn visit_choice(&mut self, this: usize, children: Vec<usize>) -> WFA {
    let mut wfa = WFA {
      can_fail: true,
      can_succeed: false,
      always_consume: true,
      never_consume: true
    };
    for i in 0..children.len() {
      let child = children[i];
      let savepoint = self.save();
      let child_wfa = self.visit_expr(child);
      self.restore(savepoint);
      wfa.can_fail &= child_wfa.can_fail;
      wfa.can_succeed |= child_wfa.can_succeed;
      wfa.always_consume &= child_wfa.always_consume;
      wfa.never_consume &= child_wfa.never_consume;
      if i != children.len() - 1 && !child_wfa.can_fail {
        self.error_unreachable_branches(this, children[i]);
        return wfa;
      }
    }
    wfa
  }

  fn visit_sequence(&mut self, _this: usize, children: Vec<usize>) -> WFA {
    let savepoint = self.save();
    let mut wfa = WFA {
      can_fail: false,
      can_succeed: true,
      always_consume: false,
      never_consume: true
    };
    for child in children {
      let child_wfa = self.visit_expr(child);
      wfa.can_fail |= child_wfa.can_fail;
      wfa.can_succeed &= child_wfa.can_succeed;
      wfa.always_consume |= child_wfa.always_consume;
      wfa.never_consume &= child_wfa.never_consume;
      if child_wfa.always_consume {
        self.consumed_input = true;
      }
    }
    self.restore(savepoint);
    wfa
  }
}
