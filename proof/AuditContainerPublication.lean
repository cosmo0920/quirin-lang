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

import QCore.ContainerPublication

/-! Card **W45-FB-RETRY-LIFETIME**: intrusive-container publication
(`resource_published` / `resource_unpublished`, contract specification §C04.8,
gateway constraint 10d).

Four directions are audited.

**A publication discharges into a caller's container.** A live identity
linked into a container reached from an unrebound formal owes nothing at the
exit; the pre-card borrowed reading accused the same body.

**A published identity stays the container's.** Releasing it is a violation
candidate whatever follows; returning it refuses; an unlink restores the
body's custody, after which a release is clean and its absence is a leak.

**Everything else refuses.** A publication into any other container, a second
publication, an unlink of an identity not published, and every step after a
refusal or a violation.

**The join.** A site is published after a join only when both paths published
it, so a one-sided publication is never unlinked.

Faithful reading of the entry and container actuals and the Rust
correspondence are trust boundary 28's premises, pinned by
`crates/qcore-c-gateway/tests/retry_lifetime.rs`. -/

-- A publication discharges into a caller's container.
#print axioms QCore.ContainerPublication.a_publication_into_a_callers_container_discharges
#print axioms QCore.ContainerPublication.the_borrowed_reading_accused_the_publication

-- A published identity stays the container's.
#print axioms QCore.ContainerPublication.a_release_while_published_is_a_violation
#print axioms QCore.ContainerPublication.a_release_while_published_is_never_safe
#print axioms QCore.ContainerPublication.an_unlink_restores_custody
#print axioms QCore.ContainerPublication.publish_unlink_release_is_safe
#print axioms QCore.ContainerPublication.an_unlink_without_a_release_leaks
#print axioms QCore.ContainerPublication.a_return_of_a_published_identity_is_refused

-- Everything else refuses.
#print axioms QCore.ContainerPublication.a_publication_into_another_container_is_refused
#print axioms QCore.ContainerPublication.an_unlink_of_an_unpublished_identity_is_refused
#print axioms QCore.ContainerPublication.only_a_live_identity_is_published
#print axioms QCore.ContainerPublication.a_published_identity_is_not_published_again
#print axioms QCore.ContainerPublication.no_step_certifies_after_a_refusal
#print axioms QCore.ContainerPublication.no_step_certifies_after_a_violation
#print axioms QCore.ContainerPublication.run_refused
#print axioms QCore.ContainerPublication.run_violation

-- The join.
#print axioms QCore.ContainerPublication.a_joined_publication_was_published_on_both
#print axioms QCore.ContainerPublication.a_one_sided_publication_is_not_unlinked
