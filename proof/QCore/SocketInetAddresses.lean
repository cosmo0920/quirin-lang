-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

/-! Declared IPv4 and IPv6 addresses with ports for `source.socket.v1`
(card `SOCKET-INET-ADDRESSES`).

Four groups of claims are proved here, and they are the four the Rust
implementation rests on.

**An address is total and has exactly one family.** Every address this route
admits belongs to exactly one of two *disjoint* spaces. Nothing here relates
`0.0.0.0` to `::`, and nothing makes a socket belong to both — which is the model
half of the `IPV6_V6ONLY` decision below.

**Resolution refuses outward, never inward.** A `connect` or a `sendto` spelling
an address reaches a socket of this same program or reaches nothing at all:
resolution can only ever return a socket some `bind` of the program actually
declared, an address no bind declares resolves to nothing, and an address naming
a **kernel-chosen** port resolves to nothing whatever the program bound — because
an ephemeral port is not known at check time and so is not a name. The implicit
`bind NAME;` form declares exactly such an address, which is why a program
written before this card cannot be reached by an address it never wrote down.

**The `IPV6_V6ONLY` decision is one value on every arm, stated as a premise.**
It is modelled as a function from the three compilation arms and proved constant
and `true`. A change to any single arm is then a *failed proof* rather than a
silent per-platform difference, which is the whole point of the decision: Linux
defaults the option off and Windows defaults it on.

**A missing address family is recorded, never replaced.** Socket creation is
modelled with three outcomes, and the theorem that matters is that creation never
yields a socket in a family other than the one asked for. A fallback from IPv6 to
IPv4 would bind a different address from the declared one, and the model says it
cannot happen.

What is **not** proved here: that the Rust reader extracts an address faithfully
from source tokens; that a platform binds what the emitted program asks it to;
or anything about a failed primitive — the emitted program exits there. Partial
correctness only, stated rather than proved away. -/

namespace QCore.SocketInetAddresses

/-! ## Addresses -/

/-- The two address families, kept **disjoint**. -/
inductive Family where
  | v4
  | v6
  deriving DecidableEq, Repr

/-- A declared address whose port the checker has already evaluated.

The host is its wire octets, because that is what the checker computes and what
the emitted program carries: there is one parse, and the text does not survive
it. -/
structure Address where
  family : Family
  host : List Nat
  port : Nat
  deriving DecidableEq, Repr

/-- An address whose port the kernel chooses. -/
def ephemeral (a : Address) : Bool := a.port == 0

/-- Every address belongs to exactly one family, and the two are distinct. -/
theorem every_address_has_exactly_one_family (a : Address) :
    a.family = Family.v4 ∨ a.family = Family.v6 := by
  cases h : a.family
  · exact Or.inl rfl
  · exact Or.inr rfl

theorem the_two_families_are_disjoint : Family.v4 ≠ Family.v6 := by
  intro h
  exact Family.noConfusion h

/-- The address a bare `bind NAME;` declares: IPv4 loopback, kernel-chosen
port. Spelled once here exactly as it is spelled once in the Rust model. -/
def implicitLocal : Address :=
  { family := Family.v4, host := [127, 0, 0, 1], port := 0 }

theorem an_implicit_bind_is_ipv4_loopback_on_a_kernel_chosen_port :
    implicitLocal.family = Family.v4 ∧ ephemeral implicitLocal = true := by
  exact ⟨rfl, rfl⟩

/-! ## Resolution

What every `bind` of the program declared, and what a target address reaches. -/

/-- The binds of one program, in program order: the socket and the address it
declared. -/
abbrev Binds := List (String × Address)

/-- The first socket of this program bound at exactly this address.

Written as its own recursion rather than through a library combinator, so every
theorem below is an ordinary induction on the program's binds. -/
def lookup (binds : Binds) (a : Address) : Option String :=
  match binds with
  | [] => none
  | (s, b) :: rest => if b = a then some s else lookup rest a

/-- The socket of this program a target address reaches, if any.

Two guards, and both are refusals: an **ephemeral** address names nothing because
its port is not known at check time, and an address no bind declared names
nothing because its peer is outside the program. -/
def reach (binds : Binds) (a : Address) : Option String :=
  if ephemeral a then none else lookup binds a

/-- **An ephemeral address is not a name.** -/
theorem an_ephemeral_address_reaches_nothing (binds : Binds) (a : Address)
    (h : ephemeral a = true) : reach binds a = none := by
  simp [reach, h]

/-- **A program that binds nothing reaches nothing.** -/
theorem no_bind_reaches_nothing (a : Address) : reach [] a = none := by
  unfold reach
  split <;> rfl

/-- Resolution returns only a pair the program actually has. -/
theorem lookup_names_a_bind (binds : Binds) (a : Address) (s : String)
    (h : lookup binds a = some s) : (s, a) ∈ binds := by
  induction binds with
  | nil => simp [lookup] at h
  | cons entry rest ih =>
    obtain ⟨name, address⟩ := entry
    by_cases hb : address = a
    · subst hb
      simp [lookup] at h
      subst h
      simp
    · simp [lookup, hb] at h
      exact List.mem_cons_of_mem _ (ih h)

/-- **Resolution never invents a socket**: whatever it returns was declared by a
`bind` of this same program, at exactly this address. This is the external-peer
refusal, stated positively. -/
theorem reaching_names_a_bind_of_this_program (binds : Binds) (a : Address)
    (s : String) (h : reach binds a = some s) : (s, a) ∈ binds := by
  unfold reach at h
  by_cases hp : ephemeral a
  · simp [hp] at h
  · simp [hp] at h
    exact lookup_names_a_bind binds a s h

/-- **A declared bind makes its address reachable**, provided it names a port. -/
theorem a_declared_bind_is_reachable_by_its_address (s : String) (a : Address)
    (rest : Binds) (h : ephemeral a = false) : reach ((s, a) :: rest) a = some s := by
  simp [reach, lookup, h]

/-- **An address no bind of this program declares reaches nothing.** The
determinism boundary, stated as a theorem about the model rather than as a
property of the diagnostic. -/
theorem an_undeclared_address_reaches_nothing (binds : Binds) (a : Address)
    (h : ∀ entry ∈ binds, entry.2 ≠ a) : reach binds a = none := by
  unfold reach
  split
  · rfl
  · induction binds with
    | nil => rfl
    | cons entry rest ih =>
      obtain ⟨name, address⟩ := entry
      have hne : address ≠ a := h (name, address) (by simp)
      simp [lookup, hne]
      exact ih (fun e he => h e (List.mem_cons_of_mem _ he))

/-- A program written before this card declares only implicit locals, and an
implicit local is ephemeral — so **no address can reach any of its sockets**. -/
theorem an_implicit_program_is_unreachable_by_address (binds : Binds)
    (h : ∀ entry ∈ binds, entry.2 = implicitLocal) (a : Address) :
    reach binds a = none := by
  by_cases hp : ephemeral a
  · simp [reach, hp]
  · apply an_undeclared_address_reaches_nothing
    intro entry hmem heq
    have himplicit : entry.2 = implicitLocal := h entry hmem
    have ha : a = implicitLocal := by rw [← heq]; exact himplicit
    subst ha
    exact hp rfl

/-! ## Families agree, or the program is refused -/

/-- Whether a socket already in `socket` may reach an endpoint in `endpoint`. -/
def agree (socket endpoint : Family) : Bool := socket == endpoint

theorem a_family_agrees_with_itself (f : Family) : agree f f = true := by
  cases f <;> rfl

/-- **The two families never stand in for each other.** -/
theorem the_families_never_stand_in_for_each_other :
    agree Family.v4 Family.v6 = false ∧ agree Family.v6 Family.v4 = false := by
  exact ⟨rfl, rfl⟩

/-- Agreement is exactly equality: there is no third answer and no coercion. -/
theorem agreement_is_equality (socket endpoint : Family) :
    agree socket endpoint = true ↔ socket = endpoint := by
  cases socket <;> cases endpoint <;> simp [agree]

/-! ## The `IPV6_V6ONLY` decision, one value on every arm -/

/-- The three compilation arms of the emitted shim. -/
inductive Arm where
  | windows
  | apple
  | posix
  deriving DecidableEq, Repr

/-- The value the emitted program sets `IPV6_V6ONLY` to, per arm.

**A premise, made checkable.** It is not read from the platform: Linux defaults
it to 0 and Windows defaults it to 1, so inheriting it would mean the same
program binds different things on different hosts. Changing any one arm below
breaks the two theorems that follow. -/
def v6only : Arm → Bool
  | Arm.windows => true
  | Arm.apple => true
  | Arm.posix => true

theorem the_v6only_decision_is_made_explicitly_on_every_arm (arm : Arm) :
    v6only arm = true := by
  cases arm <;> rfl

theorem the_v6only_decision_does_not_vary_by_arm (left right : Arm) :
    v6only left = v6only right := by
  cases left <;> cases right <;> rfl

/-! ## Creation never changes the family it was asked for -/

/-- What asking the running system for a socket in a declared family answers. -/
inductive Creation where
  /-- A socket in exactly the family asked for. -/
  | created (family : Family)
  /-- The running host explicitly does not have this family. A **recorded**
  answer, and the only one that is not a failure. -/
  | unsupported
  /-- Any other failure. -/
  | failed
  deriving DecidableEq, Repr

/-- Creation, against a host that supports some set of families. -/
def create (available : Family → Bool) (want : Family) : Creation :=
  if available want then Creation.created want
  else if want = Family.v6 then Creation.unsupported else Creation.failed

/-- **The theorem this card needs.** Creation never yields a socket in a family
other than the one the program declared: there is no fallback, so an emitted
program never binds an address in a family its source did not write down. -/
theorem creation_never_changes_the_family (available : Family → Bool)
    (want got : Family) (h : create available want = Creation.created got) :
    got = want := by
  unfold create at h
  by_cases ha : available want = true
  · simp [ha] at h
    exact h.symm
  · simp [ha] at h
    split at h <;> exact absurd h (by simp)

/-- A host without IPv6 produces the **recorded** answer, not a failure and not a
socket. -/
theorem a_missing_ipv6_is_recorded (available : Family → Bool)
    (h : available Family.v6 = false) :
    create available Family.v6 = Creation.unsupported := by
  simp [create, h]

/-- And the recorded answer is never a socket. -/
theorem an_unsupported_family_yields_no_socket (available : Family → Bool)
    (want : Family) (h : create available want = Creation.unsupported)
    (family : Family) : create available want ≠ Creation.created family := by
  rw [h]
  intro hc
  exact Creation.noConfusion hc

/-! ## A collision inside one program is decided, not deferred -/

/-- Whether a program's binds already hold this address. Port 0 never collides:
the kernel picks a different port for each. -/
def occupied (binds : Binds) (a : Address) : Bool :=
  !ephemeral a && binds.any (fun entry => entry.2 == a)

theorem two_ephemeral_binds_never_collide (binds : Binds) (a : Address)
    (h : ephemeral a = true) : occupied binds a = false := by
  simp [occupied, h]

theorem a_repeated_declared_address_collides (s : String) (a : Address)
    (rest : Binds) (h : ephemeral a = false) :
    occupied ((s, a) :: rest) a = true := by
  simp [occupied, h]

end QCore.SocketInetAddresses
