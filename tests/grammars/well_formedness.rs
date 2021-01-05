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

//! The types of the rules of this grammar must be valid (Bug #75).

use oak::oak;

oak! {

  // a = a "a" / "b"          // ERROR: left recursion

  b = "a" b "b" / "b" b       // OK

  // c = "a" c "b" / c "b"    // ERROR: left recursion

  // d = "a" . / d1           // ERROR: left recursion
  // d1 = d

  // e = "a" . / e1           // ERROR: left recursion
  // e1 = "b" . / e / "c" .

  // f = . / !f1 .            // ERROR: left recursion
  // f1 = . / "a"? f2 / "c" .
  // f2 = "b"* f

  g = . / !g1 .               // OK
  g1 = . / "a"? g2 / "c" .
  g2 = "b"+ g

  h = . / ("a" "b")+ !h1 .    // OK
  h1 = . / "a"? h / "c" .

  // i = . i1               // ERROR: left recursion
  // i1 = i1+

  // j = . j1               // ERROR: left recursion
  // j1 = ("" !j)+

  // m = !""                // ERROR: never succeed
  // n = (!"")*             // ERROR: never succeed

  // w = !(.*)              // ERROR: never succeed

  // o = (!.)*              // ERROR: loop repeat

  // p = ("a" / "b" / "")+  // ERROR loop repeat

  // q = &["a-z"]           // ERROR: loop repeat
  // q2 = q+

  // r = ["a-z"] / "A"*     // ERROR: loop repeat
  // r2 = r+

  s = &["a-z"] "a" / "A"+     // OK
  s2 = s*

  // t = ["a-z"]* / "A"+       // ERROR: unreachable branch

  // u = "a" .+ / .* / "Z" .+  // ERROR: unreachable branch

  // v = "a" .+ / "" / "Z" .+  // ERROR: unreachable branch


  // u1 = ("x"?)? // Warning useless chaining
  // u2 = ("x"?)+ // ERROR: loop repeat
  // u3 = ("x"?)* // ERROR: loop repeat
  // u4 = ("x"+)? // Warning useless chaining
  // u5 = ("x"+)* // Warning useless chaining
  // u6 = ("x"+)+ // Warning useless chaining
  // u7 = ("x"*)? // Warning useless chaining
  // u8 = ("x"*)+ // ERROR: loop repeat
  // u9 = ("x"*)* // ERROR: loop repeat

  // v1 = (&"x")+  // ERROR: loop repeat
  // v2 = (&"x")*  // ERROR: loop repeat
  // v3 = (&"x")?  // ERROR: always succeed, never consume
  // v4 = (!"x")+  // ERROR: loop repeat
  // v5 = (!"x")*  // ERROR: loop repeat
  // v6 = (!"x")?  // ERROR: always succeed, never consume
  w1 = &("x"+)
  // w2 = &("x"*)  // ERROR: always succeed, never consume
  // w3 = &("x"?)  // ERROR: always succeed, never consume
  w4 = !("x"+)
  // w5 = !("x"*)  // ERROR: never succeed
  // w6 = !("x"?)  // ERROR: never succeed
}
