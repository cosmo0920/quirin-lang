# Quirin source evidence — 2026-09-29

Archive SHA-256: `7e6e2351bcb5b83caa249870bef82d8d6d5e84e42cbacd6093ce9e179ebea507`

These are unedited source excerpts taken from the attached archive. Line numbers refer to the original Rust files, not to lines in this Markdown document. Q identifiers provide the evidence for the comparison tables. Guarantees described in comments are distinguished from execution-based proof and were read alongside the implementation's match branches. The Rust tests were not executed in the original analysis environment.

## Index

| Evidence | Source | Lines |
|---|---|---|
| Q01 | `crates/qcore-c-gateway/src/lib.rs` | 1–80 |
| Q02 | `crates/qcore-c-gateway/src/typed_c_ast.rs` | 1–151 |
| Q03 | `crates/qcore-c-gateway/src/legacy_c.rs` | 233–415 |
| Q04 | `crates/qcore-c-gateway/src/legacy_c.rs` | 1239–1342 |
| Q05 | `crates/qcore-c-gateway/src/legacy_c.rs` | 1494–1520 |
| Q06 | `crates/qcore-c-gateway/src/legacy_c.rs` | 4303–4464 |
| Q07 | `crates/qcore-c-gateway/src/legacy_c.rs` | 9245–9415 |
| Q08 | `crates/qcore-c-gateway/src/legacy_c.rs` | 9472–9790 |
| Q09 | `crates/qcore-c-gateway/src/typed_source.rs` | 1–176 |
| Q10 | `crates/qcore-c-gateway/src/typed_source.rs` | 469–579 |
| Q11 | `crates/qcore-c-gateway/src/typed_source.rs` | 582–796 |
| Q12 | `crates/qcore-c-gateway/src/typed_source.rs` | 798–960 |
| Q13 | `crates/qcore-c-gateway/src/typed_source.rs` | 994–1076 |
| Q14 | `crates/qcore-c-gateway/src/typed_source.rs` | 1315–1410 |
| Q15 | `crates/qcore-c-gateway/src/typed_source.rs` | 1595–1723 |
| Q16 | `crates/qcore-c-gateway/src/typed_access_path.rs` | 950–1182 |
| Q17 | `crates/qcore-c-gateway/src/typed_access_positions.rs` | 1–100 |
| Q18 | `crates/qcore-c-gateway/src/aggregate_store.rs` | 1–165 |
| Q19 | `crates/qcore-c-gateway/src/legacy_condition_desugar.rs` | 1–180 |
| Q20 | `crates/qcore-c-gateway/src/variadic_nested.rs` | 1–105 |
| Q21 | `crates/qcore-c-gateway/src/legacy_generic_selection.rs` | 1–91 |
| Q22 | `crates/qcore-c-gateway/src/type_query.rs` | 1–60 |
| Q23 | `crates/qcore-c-gateway/src/sizeof_operand.rs` | 1–52 |
| Q24 | `crates/qcore-c-gateway/src/index_traversal.rs` | 1–100 |
| Q25 | `crates/qcore-c-gateway/src/indexed_collection.rs` | 1–115 |
| Q26 | `crates/qcore-c-gateway/src/legacy_summary.rs` | 1–125 |
| Q27 | `crates/qcore-c-gateway/src/legacy_recursion.rs` | 1–180 |
| Q28 | `crates/qcore-c-gateway/src/legacy_declared_abi.rs` | 1–140 |
| Q29 | `crates/qcore-schemas/src/contract.rs` | 1–208 |
| Q30 | `crates/qcore-schemas/src/contract.rs` | 388–434 |
| Q31 | `crates/qcore-contract/src/contract_loader.rs` | 1–85 |
| Q32 | `crates/qcore-core/src/scalar_calls.rs` | 1–100 |
| Q33 | `crates/qcore-core/src/disjoint_footprint.rs` | 1–153 |
| Q34 | `crates/qcore-core/src/weak_memory.rs` | 1–90 |
| Q35 | `crates/qcore-c-gateway/src/atomic_publication_source.rs` | 1–100 |
| Q36 | `crates/qcore-c-gateway/tests/access_path_statements.rs` | 1–116 |
| Q37 | `crates/qcore-c-gateway/src/typed_position_shapes.rs` | 350–438 |
| Q38 | `crates/qcore-c-gateway/src/lexical_nested_assignment.rs` | 1–68 |
| Q39 | `crates/qcore-c-gateway/src/typed_root_declarations.rs` | 280–425 |
| Q40 | `crates/qcore-c-gateway/src/legacy_return_family.rs` | 1–101 |
| Q41 | `crates/qcore-c-gateway/src/loop_owner_property_keys.rs` | 1–95 |
| Q42 | `crates/qcore-c-gateway/src/cleanup_replay_source.rs` | 145–206 |
| Q43 | `crates/qcore-c-gateway/src/typed_access_path.rs` | 1220–1348 |
| Q44 | `crates/qcore-c-gateway/src/typed_field_access.rs` | 309–350 |
| Q45 | `crates/qcore-c-gateway/src/legacy_c.rs` | 1–85 |
| Q46 | `crates/qcore-core/src/concurrency.rs` | 1–85 |
| Q47 | `crates/qcore-c-gateway/tests/condition_call_desugar.rs` | 280–330 |

<a id="q01"></a>

## Q01 — `crates/qcore-c-gateway/src/lib.rs:1–80`

File SHA-256: `706e7bbc292b6971132a204965d71c2ecf3fad0cb0e79aff2af6461c65aa1f80`

```rust
    1: //! Conservative existing-C analysis and Clang transport (L2 gateway).
    2: //!
    3: //! The gateway validates preprocessing, derives and checks custody summaries,
    4: //! and rederives certificate evidence. Contracts remain explicit assumptions.
    5: //! No provider or compiler crate is a dependency. The abstract interpreter,
    6: //! summary inference machinery and entry-state constructors remain private.
    7: //!
    8: //! [`typed_c_ast`] provides raw Clang JSON transport for source-profile
    9: //! interpreters. Its output alone is never a safety certificate.
   10: 
   11: // Card W46-ACCESS-PATH-CENSUS: the analysed text of every unit, for the
   12: // typed access-path census.
   13: #[cfg(feature = "indirect-census")]
   14: pub mod access_path_census;
   15: mod aggregate_store;
   16: pub mod atomic_publication_source;
   17: pub mod boolean_source;
   18: mod borrow_liveness;
   19: mod borrowed_actual;
   20: pub mod call_use_source;
   21: mod clang;
   22: pub mod cleanup_replay_source;
   23: pub mod composition_source;
   24: #[cfg(feature = "indirect-census")]
   25: pub mod condition_call_census;
   26: pub mod counter_source;
   27: #[cfg(feature = "indirect-census")]
   28: pub mod destination_suffix_census;
   29: mod index_traversal;
   30: pub mod indexed_collection;
   31: #[cfg(feature = "indirect-census")]
   32: pub mod indirect_census;
   33: mod indirect_chain_access;
   34: #[cfg(feature = "indirect-census")]
   35: pub mod indirect_chain_census;
   36: pub mod legacy_c;
   37: pub mod legacy_census;
   38: mod legacy_condition_call;
   39: // Card W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (rule 10o).
   40: mod legacy_summary;
   41: mod lexical_nested_assignment;
   42: mod may_alias;
   43: mod neutral_position;
   44: mod publication;
   45: // Card W44-COLLECTION-ALLOCATION: unevaluated `sizeof` dereference operands.
   46: mod sizeof_operand;
   47: // Card W45-SUB-OBJECT-STORES: the sub-object store census.
   48: pub mod source_call_use;
   49: #[cfg(feature = "indirect-census")]
   50: pub mod sub_object_store_census;
   51: pub mod teardown_source;
   52: // Card W47-CONDVAR-JOIN-PUBLICATION (rule 10q): condvar publication and
   53: // join-before-release, read lexically.
   54: pub mod condvar_census;
   55: // Card W47-LOOP-OWNER-CONTRACTS (rule 10p): derived thread identity.
   56: pub mod thread_entry_census;
   57: #[cfg(feature = "indirect-census")]
   58: pub mod traversal_census;
   59: mod type_query;
   60: mod typed_access_path;
   61: // Card W47-ACCESS-PATH-POSITIONS: the typed reader in condition operands,
   62: // returns and call actuals.
   63: mod typed_access_positions;
   64: // Card W47-ACCESS-PATH-POSITION-RESIDUALS (rule 10n).
   65: mod typed_position_shapes;
   66: // Card W47-TYPED-ROOT-DECLARATION-READER (rule 10l).
   67: pub mod typed_c_ast;
   68: // Card W47-LOOP-OWNER-TYPED-DISPATCH (rule 10p amendment): the record type
   69: // of each field dispatch and store, for the thread-entry census.
   70: pub mod typed_dispatch_census;
   71: // Card W47-TYPED-CONST-POINTER-VALUE (rule 10m).
   72: mod typed_const_pointer_value;
   73: mod typed_field_access;
   74: mod typed_root_declarations;
   75: pub mod typed_source;
   76: pub mod typed_source_match;
   77: mod variadic_nested;
   78: 
   79: use qcore_contract::contract_loader;
   80: 
```

<a id="q02"></a>

## Q02 — `crates/qcore-c-gateway/src/typed_c_ast.rs:1–151`

File SHA-256: `7e4a099801d96f8ad53fa370989c5d423b03c296482accfed2fab3fdce1fce0c`

```rust
    1: //! Shared Clang typed-AST transport after the unrestricted-C gateway.
    2: //!
    3: //! Input is expected to have passed [`crate::legacy_c::validate_unrestricted_c11`].
    4: //! These helpers expose raw JSON for source-profile interpreters, not trusted
    5: //! semantic evidence. Exact shape matching and fail-closed handling of absent
    6: //! fields remain the caller's obligation.
    7: 
    8: use crate::legacy_c::C11TranslationUnit;
    9: use serde_json::Value;
   10: use std::io::Write;
   11: use std::process::Stdio;
   12: 
   13: /// Parse Clang declarations selected by its substring name filter.
   14: pub fn ast_declarations(
   15:     source: &str,
   16:     unit: &C11TranslationUnit<'_>,
   17:     function: &str,
   18: ) -> Result<Vec<Value>, String> {
   19:     let mut command = crate::clang::command()?;
   20:     unit.append_frontend_flags(&mut command);
   21:     let mut child = command
   22:         .args([
   23:             "-x",
   24:             "c",
   25:             unit.dialect.clang_argument(),
   26:             "-fsyntax-only",
   27:             "-Xclang",
   28:             "-ast-dump=json",
   29:             "-Xclang",
   30:             "-ast-dump-filter",
   31:             "-Xclang",
   32:             function,
   33:             "-",
   34:         ])
   35:         .stdin(Stdio::piped())
   36:         .stdout(Stdio::piped())
   37:         .stderr(Stdio::piped())
   38:         .spawn()
   39:         .map_err(|error| format!("Clang AST unavailable: {error}"))?;
   40:     // Drain stdout concurrently with feeding input: filtered AST output can be
   41:     // emitted before Clang has consumed a large preprocessed translation unit.
   42:     let mut stdin = child.stdin.take().ok_or("Clang stdin unavailable")?;
   43:     let output = std::thread::scope(|scope| {
   44:         let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
   45:         let output = child.wait_with_output();
   46:         let written = writer
   47:             .join()
   48:             .map_err(|_| "Clang input writer panicked".to_owned())?;
   49:         written.map_err(|error| format!("Clang input failed: {error}"))?;
   50:         output.map_err(|error| format!("Clang AST failed: {error}"))
   51:     })?;
   52:     if !output.status.success() {
   53:         return Err(format!(
   54:             "Clang AST rejected source: {}",
   55:             String::from_utf8_lossy(&output.stderr).trim()
   56:         ));
   57:     }
   58:     serde_json::Deserializer::from_slice(&output.stdout)
   59:         .into_iter::<Value>()
   60:         .map(|value| value.map_err(|error| format!("unsupported Clang AST JSON: {error}")))
   61:         .collect()
   62: }
   63: 
   64: /// Parse the complete translation-unit AST.
   65: ///
   66: /// Dedicated closed-world source judgments need this form when proving that a
   67: /// selected object has no unexamined readers or writers.  The result is still
   68: /// raw Clang transport and conveys no semantic authority by itself.
   69: pub fn translation_unit_ast(source: &str, unit: &C11TranslationUnit<'_>) -> Result<Value, String> {
   70:     translation_unit_ast_with(source, unit, |bytes| serde_json::from_slice(bytes))
   71: }
   72: 
   73: /// Run the same Clang transport while letting an internal reader deserialize
   74: /// the buffered JSON without retaining the entire translation-unit `Value`.
   75: /// The output buffer and Clang's own peak memory still exist during parsing.
   76: pub(crate) fn translation_unit_ast_with<T>(
   77:     source: &str,
   78:     unit: &C11TranslationUnit<'_>,
   79:     parse: impl FnOnce(&[u8]) -> Result<T, serde_json::Error>,
   80: ) -> Result<T, String> {
   81:     let mut command = crate::clang::command()?;
   82:     unit.append_frontend_flags(&mut command);
   83:     let mut child = command
   84:         .args([
   85:             "-x",
   86:             "c",
   87:             unit.dialect.clang_argument(),
   88:             "-fsyntax-only",
   89:             "-Xclang",
   90:             "-ast-dump=json",
   91:             "-",
   92:         ])
   93:         .stdin(Stdio::piped())
   94:         .stdout(Stdio::piped())
   95:         .stderr(Stdio::piped())
   96:         .spawn()
   97:         .map_err(|error| format!("Clang AST unavailable: {error}"))?;
   98:     let mut stdin = child.stdin.take().ok_or("Clang stdin unavailable")?;
   99:     let output = std::thread::scope(|scope| {
  100:         let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
  101:         let output = child.wait_with_output();
  102:         let written = writer
  103:             .join()
  104:             .map_err(|_| "Clang input writer panicked".to_owned())?;
  105:         written.map_err(|error| format!("Clang input failed: {error}"))?;
  106:         output.map_err(|error| format!("Clang AST failed: {error}"))
  107:     })?;
  108:     if !output.status.success() {
  109:         return Err(format!(
  110:             "Clang AST rejected source: {}",
  111:             String::from_utf8_lossy(&output.stderr).trim()
  112:         ));
  113:     }
  114:     parse(&output.stdout).map_err(|error| format!("unsupported Clang AST JSON: {error}"))
  115: }
  116: 
  117: /// Require exactly one body-bearing definition with the exact requested name.
  118: pub fn function_ast(
  119:     source: &str,
  120:     unit: &C11TranslationUnit<'_>,
  121:     function: &str,
  122: ) -> Result<Value, String> {
  123:     // Clang's name filter is a substring filter. Require one exact definition.
  124:     let mut definitions = ast_declarations(source, unit, function)?
  125:         .into_iter()
  126:         .filter(|value| {
  127:             kind(value) == "FunctionDecl"
  128:                 && value["name"].as_str() == Some(function)
  129:                 && children(value)
  130:                     .iter()
  131:                     .any(|node| kind(node) == "CompoundStmt")
  132:         })
  133:         .collect::<Vec<_>>();
  134:     if definitions.len() != 1 {
  135:         return Err(format!(
  136:             "expected one selected definition after preprocessing, found {}",
  137:             definitions.len()
  138:         ));
  139:     }
  140:     Ok(definitions.remove(0))
  141: }
  142: 
  143: /// Read a raw node kind; absent or malformed kinds yield the empty string.
  144: pub fn kind(node: &Value) -> &str {
  145:     node["kind"].as_str().unwrap_or("")
  146: }
  147: 
  148: /// Read raw child nodes; absent or malformed arrays yield an empty slice.
  149: pub fn children(node: &Value) -> &[Value] {
  150:     node["inner"].as_array().map_or(&[], Vec::as_slice)
  151: }
```

<a id="q03"></a>

## Q03 — `crates/qcore-c-gateway/src/legacy_c.rs:233–415`

File SHA-256: `37944d1ddd09ff9edcdd5cebb6807166f0e489e91771cf7f2948e253e49ef9a5`

```rust
  233: pub enum LegacyInstruction {
  234:     Acquire {
  235:         destination: String,
  236:         site: usize,
  237:     },
  238:     Alias {
  239:         destination: String,
  240:         source: String,
  241:     },
  242:     Clear {
  243:         destination: String,
  244:     },
  245:     Release {
  246:         source: String,
  247:     },
  248:     Escape {
  249:         source: String,
  250:     },
  251:     /// Transfer a returned local resource when the expression names one.
  252:     /// Non-resource scalar returns remain resource-neutral.
  253:     EscapeIfOwned {
  254:         source: String,
  255:     },
  256:     ContractCall {
  257:         function: String,
  258:         destination: Option<String>,
  259:         arguments: Vec<Option<String>>,
  260:         contract: ResourceFunctionContract,
  261:         produced_sites: Vec<usize>,
  262:         /// The admitted **bound reads** of this call (card O04-READ-BOUND): the
  263:         /// canonical `BASE->FIELD` places occupying `Borrowed` positions that
  264:         /// [`crate::aggregate_store::bound_read_actuals`] admitted on typed
  265:         /// premises alone. [`transfer`] additionally requires each one, *here*,
  266:         /// to resolve to a live aggregate this body acquired carrying the exact
  267:         /// field link an admitted store recorded; otherwise the whole call
  268:         /// refuses. Empty for every other statement in the corpus.
  269:         aggregate_reads: Vec<String>,
  270:         /// The subset of `aggregate_reads` at positions that *take* custody
  271:         /// (card O04-B09B10). The call's own contract effect releases or
  272:         /// consumes the linked identity; afterwards the member no longer holds
  273:         /// anything this projection may denote, so [`transfer`] drops both the
  274:         /// place binding and the field link. A second read of the same member
  275:         /// then fails `bound_read_resolves` rather than releasing twice.
  276:         aggregate_consumed: Vec<String>,
  277:         /// The bases of this call's admitted **member-address** actuals whose
  278:         /// aggregate *this* body bound (card O04-B09B10, O00 blockers
  279:         /// **B09**/**B10**). Handing a callee the address of a member of a
  280:         /// released, escaped, ambiguous, nullable or caller-held aggregate is
  281:         /// refused by [`transfer`] before any effect of the call applies, in
  282:         /// exactly the way a store through such an aggregate is. A
  283:         /// member-address actual on a C02-P untracked base carries no entry
  284:         /// here: that base can never bind a tracked identity at all.
  285:         aggregate_addresses: Vec<String>,
  286:         /// The C02-P-ALIAS may-alias obligations of this call's *read*
  287:         /// arguments: one entry per distinct root name a modeled position
  288:         /// dereferences that shares a declared record-pointer type with some
  289:         /// other declared name of this function. [`transfer`] refuses the read
  290:         /// when one of those peers was released on the path and neither
  291:         /// discharge applies. Empty for every argument the rule does not
  292:         /// relate, which is most of the corpus. See [`crate::may_alias`].
  293:         alias_reads: Vec<AliasPeers>,
  294:         /// Cards **O04-TRACE** and **B04-NESTED-CALL-ARGUMENT**: every sibling
  295:         /// nested call this statement admits. Each declared effect applies
  296:         /// before this call's own.
  297:         ///
  298:         /// It is admitted only when its declared effects are the *identity* on
  299:         /// the abstract state — result `None`, every parameter `Borrowed` — so
  300:         /// applying it first is well defined despite C leaving the evaluation
  301:         /// order of arguments unspecified. Every other nested call is refused
  302:         /// where it stands, with card O04-B09B10's diagnostic.
  303:         nested: Vec<NestedContractCall>,
  304:         /// Card **W45-FB-RETRY-LIFETIME**: the operands of the call's declared
  305:         /// container effects, applied after its borrowed reading
  306:         /// ([`container_publication`]). `None` for every call whose contract
  307:         /// in force has none.
  308:         container: Option<Box<container_publication::Operands>>,
  309:     },
  310:     BranchOnNull {
  311:         source: String,
  312:         true_is_null: bool,
  313:     },
  314:     /// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: a branch on the validity of
  315:     /// a name that may hold an integer-carried identity (`fd < 0`,
  316:     /// `fd == -1`, `fd >= 0`, …). It is the integer counterpart of
  317:     /// [`Self::BranchOnNull`]: an `Invalid` edge carries no resource at the
  318:     /// tested identity, a `Valid` edge carries a valid one, and an `Unrefined`
  319:     /// edge carries the state unchanged. A null-shaped test of such a name
  320:     /// (`if (fd)`, `fd == 0`) is one of these with both edges `Unrefined`,
  321:     /// because descriptor `0` is valid. Emitted only for a name
  322:     /// [`descriptor_custody::DescriptorNames`] lists.
  323:     BranchOnValidity {
  324:         source: String,
  325:         when_true: ValidityFact,
  326:         when_false: ValidityFact,
  327:     },
  328:     /// A branch on `LEFT == RIGHT` or `LEFT != RIGHT` between two plain names
  329:     /// this function declares with the same record-pointer type (C02-P-ALIAS).
  330:     ///
  331:     /// It is *not* a null test — `BranchOnNull` still claims every one of
  332:     /// those — and it refines nothing about custody. Its only effect is that
  333:     /// the edge on which the two names are provably distinct carries that fact,
  334:     /// which discharges the may-alias obligation for exactly that pair until
  335:     /// either name is rebound. On the other edge nothing at all is assumed.
  336:     BranchOnDistinct {
  337:         left: String,
  338:         right: String,
  339:         /// Whether the first successor is the edge on which the two names are
  340:         /// distinct (`!=`), rather than the edge on which they are equal.
  341:         true_is_distinct: bool,
  342:     },
  343:     /// Bind `destination` to a value this function does not track: the read of
  344:     /// an admitted direct member of an untracked aggregate (C02-P, O04).
  345:     ///
  346:     /// The resulting abstract pointer is exactly the one an [`Self::Alias`]
  347:     /// from an unbound place would produce — unknown and possibly null, so
  348:     /// every downstream use stays fail-closed. The only difference is that no
  349:     /// unresolved-alias diagnostic is reported, because the typed premises of
  350:     /// [`crate::borrowed_actual`] establish that the source place is one this
  351:     /// function never binds. It never creates, releases or transfers custody.
  352:     OpaqueValue {
  353:         destination: String,
  354:         /// The C02-P-ALIAS may-alias obligation of the base this read
  355:         /// dereferences, when the rule relates it to another declared name.
  356:         /// `None` whenever it relates nothing, which is the pre-existing
  357:         /// answer.
  358:         alias_read: Option<AliasPeers>,
  359:     },
  360:     /// Forget a declared scalar's old branch fact after a modeled write.
  361:     /// An absent binding admits both scalar branches and carries no pointer
  362:     /// identity. Resource obligations are retained independently.
  363:     ForgetScalar {
  364:         destination: String,
  365:     },
  366:     /// An exactly-shaped scalar store through a record pointer this function
  367:     /// may bind.  Unlike the unbound scalar-field no-op, transfer must prove
  368:     /// that `base` denotes one settled, live, non-null identity with no
  369:     /// unresolved effect before the store is admitted.  The scalar member and
  370:     /// scalar-only right-hand side carry no tracked identity, so both an
  371:     /// accepted store and a refusal carrying `UnknownAlias` preserve the
  372:     /// pointer, custody, field-link and scalar-fact maps.
  373:     ScalarMemberStore {
  374:         base: String,
  375:         /// The same C02-P-ALIAS obligation as every other modeled read through
  376:         /// a declared record pointer.
  377:         alias_read: Option<AliasPeers>,
  378:     },
  379:     /// Card **W43-INDIRECT-ACCESS-GENERAL**: one complete statement whose
  380:     /// every memory access is a typed scalar access (`*B`, `B[I]`, `B->F`,
  381:     /// `S.F`) and whose destination and value are scalar; see
  382:     /// [`crate::typed_field_access::typed_scalar_statement`].
  383:     ///
  384:     /// It is custody-neutral: it acquires, releases, transfers and rebinds no
  385:     /// tracked identity, so every projection map is preserved, admitted or
  386:     /// refused. What it adds is the dereference obligation of each base,
  387:     /// checked by [`transfer`]: a base must be unbound on every reaching path,
  388:     /// a non-null binding with no tracked target, or exactly one known,
  389:     /// non-null, live identity with no nullable or unresolved-effect marker.
  390:     /// Anything else records `UnknownAlias`. A base settled on one released
  391:     /// identity is also a `use_after_release` candidate, reported from the
  392:     /// settled states. Bounds are not decided: a subscript's index is read as
  393:     /// a scalar and nothing is claimed about the object's extent.
  394:     IndirectScalarAccess {
  395:         bases: Vec<String>,
  396:         /// The C02-P-ALIAS obligations of the bases read through `->`.
  397:         alias_reads: Vec<AliasPeers>,
  398:         /// The declared scalar variable written, whose old scalar fact is
  399:         /// forgotten exactly as [`Self::ForgetScalar`] forgets it. Card
  400:         /// W45-FB-BACKLOG-CUSTODY also names here an admitted list cursor or
  401:         /// element (`list_iteration`): its value is list memory this function
  402:         /// does not track, so it too is left unbound.
  403:         destination: Option<String>,
  404:         /// For a write through `*B` or `B[I]` only: the bindings it may have
  405:         /// rewritten, bounded exactly as the refusal it replaces bounded them
  406:         /// ([`RefusedWrites`]). A character-typed write may rewrite a pointer
  407:         /// object's representation, so every binding outside this frame is
  408:         /// made unknown and possibly null, as before; the resource, custody
  409:         /// and field-link maps are untouched. `None` for a read or a member
  410:         /// store, which writes one scalar and nothing else.
  411:         frame: Option<RefusedWrites>,
  412:         /// Card **W44-INDIRECT-CHAIN-ACCESS**: the record-pointer member
  413:         /// chains the statement or condition reads or writes through, each
  414:         /// with its per-link obligation
  415:         /// ([`crate::indirect_chain_access::chain_resolves`]). A dereferenced
```

<a id="q04"></a>

## Q04 — `crates/qcore-c-gateway/src/legacy_c.rs:1239–1342`

File SHA-256: `37944d1ddd09ff9edcdd5cebb6807166f0e489e91771cf7f2948e253e49ef9a5`

```rust
 1239: pub enum CFrontendFlag {
 1240:     AllWarnings,
 1241:     ExtraWarnings,
 1242:     /// Warn when an undefined preprocessor identifier starts with this prefix.
 1243:     UndefPrefix(String),
 1244:     DebugInformation,
 1245:     PositionIndependentCode,
 1246:     AssemblerNoExecStack,
 1247:     /// Select one Apple Clang architecture using its two-argument `-arch` form.
 1248:     Architecture(String),
 1249:     // --- Diagnostic-only warning selections -------------------------------
 1250:     // Each of the members below selects or deselects one Clang warning group.
 1251:     // Warning selection is measured to leave the preprocessed token stream
 1252:     // byte-identical and the predefined macro set unchanged, so transporting
 1253:     // one preserves the recorded compile command without changing the analysed
 1254:     // program. None of them is `-Werror`: no warning becomes a hard error, so
 1255:     // an admitted unit cannot be turned into a refusal by its warning set.
 1256:     /// `-Wstrict-aliasing`
 1257:     StrictAliasingWarnings,
 1258:     /// `-Wstrict-prototypes`
 1259:     StrictPrototypes,
 1260:     /// `-Wundef`
 1261:     UndefinedIdentifierWarnings,
 1262:     /// `-Wmissing-prototypes`
 1263:     MissingPrototypes,
 1264:     /// `-Winit-self`
 1265:     InitSelf,
 1266:     /// `-Wmissing-field-initializers`
 1267:     MissingFieldInitializers,
 1268:     /// `-Wdeclaration-after-statement`
 1269:     DeclarationAfterStatement,
 1270:     /// `-Waddress`
 1271:     AddressComparisons,
 1272:     /// `-Woverride-init`
 1273:     OverrideInit,
 1274:     /// `-Wwrite-strings`
 1275:     WriteStrings,
 1276:     /// `-Wvla`
 1277:     VariableLengthArrays,
 1278:     /// `-Wno-unused-parameter`
 1279:     NoUnusedParameter,
 1280:     /// `-Wno-unused-function`
 1281:     NoUnusedFunction,
 1282:     /// `-Wno-pragmas`
 1283:     NoPragmas,
 1284:     /// `-Wno-void-pointer-to-enum-cast`
 1285:     NoVoidPointerToEnumCast,
 1286:     // --- Code generation ---------------------------------------------------
 1287:     /// `-fno-strict-aliasing`: relaxes the aliasing rule the optimizer may
 1288:     /// assume. Measured to change neither the predefined macros nor the
 1289:     /// preprocessed text; it is transported so the recorded command is
 1290:     /// reproduced, and, exactly like [`Self::AssemblerNoExecStack`], no object
 1291:     /// or aliasing property is certified by admitting it.
 1292:     NoStrictAliasing,
 1293:     /// Transport the native observer's section layout flags unchanged.
 1294:     FunctionSections,
 1295:     DataSections,
 1296:     /// Exact bounded sanitizer selection. This affects `__has_feature`, so
 1297:     /// dropping it would analyze a different preprocessor configuration.
 1298:     AddressUndefinedSanitizers,
 1299:     /// `-O0` … `-Og`: transported because the level is visible to the
 1300:     /// preprocessor (see [`COptimizationLevel`]).
 1301:     Optimization(COptimizationLevel),
 1302: }
 1303: 
 1304: impl CFrontendFlag {
 1305:     fn append_to(&self, command: &mut std::process::Command) {
 1306:         match self {
 1307:             Self::AllWarnings => command.arg("-Wall"),
 1308:             Self::ExtraWarnings => command.arg("-Wextra"),
 1309:             Self::UndefPrefix(prefix) => command.arg(format!("-Wundef-prefix={prefix}")),
 1310:             Self::DebugInformation => command.arg("-g"),
 1311:             Self::PositionIndependentCode => command.arg("-fPIC"),
 1312:             Self::AssemblerNoExecStack => command.arg("-Wa,--noexecstack"),
 1313:             Self::Architecture(architecture) => command.args(["-arch", architecture]),
 1314:             Self::StrictAliasingWarnings => command.arg("-Wstrict-aliasing"),
 1315:             Self::StrictPrototypes => command.arg("-Wstrict-prototypes"),
 1316:             Self::UndefinedIdentifierWarnings => command.arg("-Wundef"),
 1317:             Self::MissingPrototypes => command.arg("-Wmissing-prototypes"),
 1318:             Self::InitSelf => command.arg("-Winit-self"),
 1319:             Self::MissingFieldInitializers => command.arg("-Wmissing-field-initializers"),
 1320:             Self::DeclarationAfterStatement => command.arg("-Wdeclaration-after-statement"),
 1321:             Self::AddressComparisons => command.arg("-Waddress"),
 1322:             Self::OverrideInit => command.arg("-Woverride-init"),
 1323:             Self::WriteStrings => command.arg("-Wwrite-strings"),
 1324:             Self::VariableLengthArrays => command.arg("-Wvla"),
 1325:             Self::NoUnusedParameter => command.arg("-Wno-unused-parameter"),
 1326:             Self::NoUnusedFunction => command.arg("-Wno-unused-function"),
 1327:             Self::NoPragmas => command.arg("-Wno-pragmas"),
 1328:             Self::NoVoidPointerToEnumCast => command.arg("-Wno-void-pointer-to-enum-cast"),
 1329:             Self::NoStrictAliasing => command.arg("-fno-strict-aliasing"),
 1330:             Self::FunctionSections => command.arg("-ffunction-sections"),
 1331:             Self::DataSections => command.arg("-fdata-sections"),
 1332:             Self::AddressUndefinedSanitizers => command.arg("-fsanitize=address,undefined"),
 1333:             Self::Optimization(level) => command.arg(level.spelling()),
 1334:         };
 1335:     }
 1336: }
 1337: 
 1338: impl C11TranslationUnit<'_> {
 1339:     pub(crate) fn append_frontend_flags(&self, command: &mut std::process::Command) {
 1340:         for option in &self.preprocessor_options {
 1341:             if let CPreprocessorOption::FrontendFlag(flag) = option {
 1342:                 flag.append_to(command);
```

<a id="q05"></a>

## Q05 — `crates/qcore-c-gateway/src/legacy_c.rs:1494–1520`

File SHA-256: `37944d1ddd09ff9edcdd5cebb6807166f0e489e91771cf7f2948e253e49ef9a5`

```rust
 1494: pub enum CCompilerDialect {
 1495:     #[default]
 1496:     C11,
 1497:     Gnu99,
 1498:     Gnu11,
 1499:     Gnu17,
 1500: }
 1501: 
 1502: impl CCompilerDialect {
 1503:     pub(crate) fn clang_argument(self) -> &'static str {
 1504:         match self {
 1505:             Self::C11 => "-std=c11",
 1506:             Self::Gnu99 => "-std=gnu99",
 1507:             Self::Gnu11 => "-std=gnu11",
 1508:             Self::Gnu17 => "-std=gnu17",
 1509:         }
 1510:     }
 1511: }
 1512: 
 1513: /// Selects whether header-expanded definitions participate in the proof.
 1514: #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
 1515: pub enum LegacyFunctionScope {
 1516:     /// Preserve the Cycle 17 behavior and analyze every preprocessed body.
 1517:     #[default]
 1518:     PreprocessedTranslationUnit,
 1519:     /// Analyze only functions whose definitions occur in the input source.
 1520:     MainSourceDefinitions,
```

<a id="q06"></a>

## Q06 — `crates/qcore-c-gateway/src/legacy_c.rs:4303–4464`

File SHA-256: `37944d1ddd09ff9edcdd5cebb6807166f0e489e91771cf7f2948e253e49ef9a5`

```rust
 4303: enum Statement {
 4304:     Instruction(LegacyInstruction, usize),
 4305:     Block(Vec<Statement>),
 4306:     If {
 4307:         line: usize,
 4308:         condition_supported: bool,
 4309:         /// Card **B02-BRANCH-CONDITION**: the one contracted call whose
 4310:         /// custody effects are evaluated before this branch decision.  The
 4311:         /// isolated instruction is produced by the ordinary statement
 4312:         /// classifier; `Some` therefore adds no second call-effect model.
 4313:         condition_call: Option<Box<LegacyInstruction>>,
 4314:         /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
 4315:         /// lowers to when its every memory access is a typed scalar access
 4316:         /// (see [`Parser::condition_access`]). Consulted only where the
 4317:         /// existing lowering would have been the refusal.
 4318:         condition_access: Option<Box<LegacyInstruction>>,
 4319:         writes: RefusedWrites,
 4320:         null_condition: Option<PointerNullCondition>,
 4321:         /// C02-P-ALIAS: the pointer-inequality guard this condition expresses,
 4322:         /// when it relates two names the may-alias rule relates. Consulted only
 4323:         /// when `null_condition` is `None`, so no null test changes shape.
 4324:         distinct_condition: Option<PointerDistinctCondition>,
 4325:         /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the element null test this
 4326:         /// refused condition expresses, when its premises hold. Consulted only
 4327:         /// when the condition is otherwise refused.
 4328:         element_condition: Option<(String, crate::indexed_collection::IndexSpelling, bool)>,
 4329:         /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
 4330:         /// branch edges of a call-bearing condition every route above refused.
 4331:         condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
 4332:         then_body: Vec<Statement>,
 4333:         else_body: Vec<Statement>,
 4334:     },
 4335:     While {
 4336:         line: usize,
 4337:         condition_supported: bool,
 4338:         /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated at
 4339:         /// every visit to this loop header, before either successor is chosen.
 4340:         condition_call: Option<Box<LegacyInstruction>>,
 4341:         /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
 4342:         /// lowers to when its every memory access is a typed scalar access
 4343:         /// (see [`Parser::condition_access`]). Consulted only where the
 4344:         /// existing lowering would have been the refusal.
 4345:         condition_access: Option<Box<LegacyInstruction>>,
 4346:         writes: RefusedWrites,
 4347:         null_condition: Option<PointerNullCondition>,
 4348:         /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
 4349:         /// branch edges of a call-bearing condition every route above refused.
 4350:         condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
 4351:         body: Vec<Statement>,
 4352:     },
 4353:     For {
 4354:         line: usize,
 4355:         /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the index and bound of a
 4356:         /// canonical counted loop around at least one element operation.
 4357:         counted: Option<(String, crate::indexed_collection::Term)>,
 4358:         init: Option<Box<(LegacyInstruction, usize)>>,
 4359:         condition_always_true: bool,
 4360:         condition_supported: bool,
 4361:         /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated at
 4362:         /// every visit to this loop header, before either successor is chosen.
 4363:         condition_call: Option<Box<LegacyInstruction>>,
 4364:         /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
 4365:         /// lowers to when its every memory access is a typed scalar access
 4366:         /// (see [`Parser::condition_access`]). Consulted only where the
 4367:         /// existing lowering would have been the refusal.
 4368:         condition_access: Option<Box<LegacyInstruction>>,
 4369:         writes: RefusedWrites,
 4370:         null_condition: Option<PointerNullCondition>,
 4371:         /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
 4372:         /// branch edges of a call-bearing condition every route above refused.
 4373:         condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
 4374:         increment: Option<Box<(LegacyInstruction, usize)>>,
 4375:         /// Card **W45-FB-BACKLOG-CUSTODY**: a second increment node, run after
 4376:         /// `increment` and before the header (a safe list iteration's
 4377:         /// `N = C->next`). `None` for every other loop.
 4378:         list_step: Option<Box<(LegacyInstruction, usize)>>,
 4379:         body: Vec<Statement>,
 4380:     },
 4381:     DoWhile {
 4382:         line: usize,
 4383:         condition_supported: bool,
 4384:         /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated
 4385:         /// after every completed body execution, before the back/exit choice.
 4386:         condition_call: Option<Box<LegacyInstruction>>,
 4387:         /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
 4388:         /// lowers to when its every memory access is a typed scalar access
 4389:         /// (see [`Parser::condition_access`]). Consulted only where the
 4390:         /// existing lowering would have been the refusal.
 4391:         condition_access: Option<Box<LegacyInstruction>>,
 4392:         /// The controlling expression is the single literal token `0`, so the
 4393:         /// body runs exactly once (C11 §6.8.5.2) and the CFG carries no back
 4394:         /// edge. Keyed on the *token*, never on an evaluated expression
 4395:         /// (standing constraint 5a: an operand-class marker is never a
 4396:         /// computed value). Card `C-GATEWAY-CONSTANT-DO-WHILE`.
 4397:         condition_is_literal_zero: bool,
 4398:         writes: RefusedWrites,
 4399:         null_condition: Option<PointerNullCondition>,
 4400:         /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
 4401:         /// branch edges of a call-bearing condition every route above refused.
 4402:         condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
 4403:         body: Vec<Statement>,
 4404:     },
 4405:     Switch {
 4406:         line: usize,
 4407:         expression_supported: bool,
 4408:         /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated
 4409:         /// once before control enters the selected switch arm.
 4410:         expression_call: Option<Box<LegacyInstruction>>,
 4411:         /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused switch expression
 4412:         /// lowers to when its every memory access is a typed scalar access
 4413:         /// (see [`Parser::expression_access`]). Consulted only where the
 4414:         /// existing lowering would have been the refusal.
 4415:         expression_access: Option<Box<LegacyInstruction>>,
 4416:         writes: RefusedWrites,
 4417:         /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls of a
 4418:         /// call-bearing switch expression every route above refused.
 4419:         expression_plan: Option<Box<condition_desugar::ConditionPlan>>,
 4420:         body: Vec<Statement>,
 4421:     },
 4422:     Case {
 4423:         line: usize,
 4424:         expression_supported: bool,
 4425:         writes: RefusedWrites,
 4426:         is_default: bool,
 4427:     },
 4428:     Label {
 4429:         line: usize,
 4430:         name: String,
 4431:     },
 4432:     Goto {
 4433:         line: usize,
 4434:         label: String,
 4435:     },
 4436:     Break {
 4437:         line: usize,
 4438:     },
 4439:     Continue {
 4440:         line: usize,
 4441:     },
 4442:     Return {
 4443:         line: usize,
 4444:         expression_supported: bool,
 4445:         writes: RefusedWrites,
 4446:         returned_place: Option<String>,
 4447:         /// A modeled evaluation of this `return` expression. Card
 4448:         /// C-GATEWAY-RETURN-CALL-W4 supplies the exact contract-call case;
 4449:         /// B01-RETURN-EXPRESSION adds bounded scalar and opaque evaluations.
 4450:         /// A declined evaluation keeps the existing refusal byte for byte.
 4451:         returned_evaluation: Option<Box<LegacyInstruction>>,
 4452:         /// Whether the evaluated value can carry custody and therefore needs
 4453:         /// the synthetic `EscapeIfOwned` exit. B01's scalar/opaque family is
 4454:         /// resource-free and uses `NoOp` instead.
 4455:         returned_evaluation_escapes: bool,
 4456:         /// Card W43-RESULT-ALIAS-RETURNED: the expression is a null pointer
 4457:         /// constant. Set only by a parser built for derived-result validation
 4458:         /// ([`Parser::admit_derived_returns`]); always `false` otherwise, so
 4459:         /// every other CFG is unchanged.
 4460:         returns_null_constant: bool,
 4461:     },
 4462: }
 4463: 
 4464: #[derive(Clone, Debug)]
```

<a id="q07"></a>

## Q07 — `crates/qcore-c-gateway/src/legacy_c.rs:9245–9415`

File SHA-256: `37944d1ddd09ff9edcdd5cebb6807166f0e489e91771cf7f2948e253e49ef9a5`

```rust
 9245:             Statement::DoWhile {
 9246:                 line,
 9247:                 writes,
 9248:                 condition_supported,
 9249:                 condition_call,
 9250:                 condition_access,
 9251:                 condition_is_literal_zero,
 9252:                 null_condition,
 9253:                 condition_plan,
 9254:                 body,
 9255:             } => {
 9256:                 // Card W44-BRANCH-CONDITION-CALLS. A call-bearing condition is
 9257:                 // never the literal `0`, so the constant rule never applies.
 9258:                 if let Some(plan) = condition_plan {
 9259:                     return self.compile_planned_do_while(plan, *line, body, continuation);
 9260:                 }
 9261:                 let instruction = condition_call.as_ref().map_or_else(
 9262:                     || {
 9263:                         branch_instruction(
 9264:                             *condition_supported,
 9265:                             null_condition,
 9266:                             condition_access.as_deref(),
 9267:                             writes,
 9268:                             "unmodeled call, aggregate or indirect access in do-while condition",
 9269:                         )
 9270:                     },
 9271:                     |call| (**call).clone(),
 9272:                 );
 9273:                 let header = self.push(instruction, Vec::new(), *line, false);
 9274:                 self.break_targets.push(continuation);
 9275:                 self.continue_targets.push(header);
 9276:                 let body_entry = self.compile_sequence(body, header);
 9277:                 self.continue_targets.pop();
 9278:                 self.break_targets.pop();
 9279:                 // `do S while (0)` executes S exactly once (C11 §6.8.5.2), so
 9280:                 // the controlling expression has the single successor
 9281:                 // `continuation`: the CFG is sequential composition, and the
 9282:                 // back edge to `body_entry` would be a path the C does not
 9283:                 // have. Removing it is the admitting direction (invariant 1),
 9284:                 // which is why the key is the literal token and the rule is
 9285:                 // proved in `QCore.ConstantDoWhile`. A `continue` in the body
 9286:                 // still targets `header`, which is exactly C's "jump to the
 9287:                 // controlling expression", and so now exits.
 9288:                 self.nodes[header].successors = if *condition_is_literal_zero {
 9289:                     vec![continuation]
 9290:                 } else {
 9291:                     vec![body_entry, continuation]
 9292:                 };
 9293:                 // SCALAR-ROUTE-EXIT-EDGE. Under card
 9294:                 // `C-GATEWAY-CONSTANT-DO-WHILE`'s rule the continuation is the
 9295:                 // header's only successor, so it is index 0; otherwise index 1,
 9296:                 // as for `while`. Either way it is the edge that leaves.
 9297:                 self.nodes[header].loop_exit_edge = Some(usize::from(!*condition_is_literal_zero));
 9298:                 body_entry
 9299:             }
 9300:             Statement::Switch {
 9301:                 line,
 9302:                 writes,
 9303:                 expression_supported,
 9304:                 expression_call,
 9305:                 expression_access,
 9306:                 expression_plan,
 9307:                 body,
 9308:             } => {
 9309:                 // Card W44-BRANCH-CONDITION-CALLS.
 9310:                 if let Some(plan) = expression_plan {
 9311:                     return self.compile_planned_switch(plan, *line, body, continuation);
 9312:                 }
 9313:                 self.break_targets.push(continuation);
 9314:                 self.switch_cases.push(SwitchTargets::default());
 9315:                 let _body_entry = self.compile_sequence(body, continuation);
 9316:                 let targets = self.switch_cases.pop().expect("switch target scope");
 9317:                 self.break_targets.pop();
 9318:                 let mut successors = targets.entries;
 9319:                 if !targets.has_default || successors.is_empty() {
 9320:                     successors.push(continuation);
 9321:                 }
 9322:                 successors.sort_unstable();
 9323:                 successors.dedup();
 9324:                 let instruction = if let Some(call) = expression_call {
 9325:                     (**call).clone()
 9326:                 } else if *expression_supported {
 9327:                     LegacyInstruction::NoOp
 9328:                 } else if let Some(access) = expression_access {
 9329:                     (**access).clone()
 9330:                 } else {
 9331:                     LegacyInstruction::Unknown {
 9332:                         description:
 9333:                             "unmodeled call, aggregate or indirect access in switch expression"
 9334:                                 .to_owned(),
 9335:                         writes: writes.clone(),
 9336:                     }
 9337:                 };
 9338:                 self.push(instruction, successors, *line, false)
 9339:             }
 9340:             Statement::Case {
 9341:                 line,
 9342:                 writes,
 9343:                 expression_supported,
 9344:                 is_default,
 9345:             } => {
 9346:                 let instruction = if *expression_supported {
 9347:                     LegacyInstruction::NoOp
 9348:                 } else {
 9349:                     LegacyInstruction::Unknown {
 9350:                         description:
 9351:                             "unmodeled call, aggregate or indirect access in case expression"
 9352:                                 .to_owned(),
 9353:                         writes: writes.clone(),
 9354:                     }
 9355:                 };
 9356:                 let entry = self.push(instruction, vec![continuation], *line, false);
 9357:                 if let Some(targets) = self.switch_cases.last_mut() {
 9358:                     targets.entries.push(entry);
 9359:                     targets.has_default |= *is_default;
 9360:                 } else {
 9361:                     self.nodes[entry].instruction = LegacyInstruction::Unknown {
 9362:                         description: "case label outside switch".to_owned(),
 9363:                         writes: RefusedWrites::Unbounded,
 9364:                     };
 9365:                 }
 9366:                 entry
 9367:             }
 9368:             Statement::Label { line, name } => {
 9369:                 let entry = self.push(LegacyInstruction::NoOp, vec![continuation], *line, false);
 9370:                 if self.labels.insert(name.clone(), entry).is_some() {
 9371:                     self.nodes[entry].instruction = LegacyInstruction::Unknown {
 9372:                         description: format!("duplicate label `{name}`"),
 9373:                         writes: RefusedWrites::Unbounded,
 9374:                     };
 9375:                 }
 9376:                 entry
 9377:             }
 9378:             Statement::Goto { line, label } => {
 9379:                 let node = self.push(LegacyInstruction::NoOp, Vec::new(), *line, false);
 9380:                 if let Some(target) = self.labels.get(label).copied() {
 9381:                     self.nodes[node].successors = vec![target];
 9382:                 } else {
 9383:                     self.pending_gotos.push((node, label.clone(), continuation));
 9384:                 }
 9385:                 node
 9386:             }
 9387:             Statement::Break { line } => {
 9388:                 if let Some(target) = self.break_targets.last().copied() {
 9389:                     self.push(LegacyInstruction::NoOp, vec![target], *line, false)
 9390:                 } else {
 9391:                     self.push(
 9392:                         LegacyInstruction::Unknown {
 9393:                             description: "break outside loop or switch".to_owned(),
 9394:                             writes: RefusedWrites::Unbounded,
 9395:                         },
 9396:                         vec![continuation],
 9397:                         *line,
 9398:                         false,
 9399:                     )
 9400:                 }
 9401:             }
 9402:             Statement::Continue { line } => {
 9403:                 if let Some(target) = self.continue_targets.last().copied() {
 9404:                     self.push(LegacyInstruction::NoOp, vec![target], *line, false)
 9405:                 } else {
 9406:                     self.push(
 9407:                         LegacyInstruction::Unknown {
 9408:                             description: "continue outside loop".to_owned(),
 9409:                             writes: RefusedWrites::Unbounded,
 9410:                         },
 9411:                         vec![continuation],
 9412:                         *line,
 9413:                         false,
 9414:                     )
 9415:                 }
```

<a id="q08"></a>

## Q08 — `crates/qcore-c-gateway/src/legacy_c.rs:9472–9790`

File SHA-256: `37944d1ddd09ff9edcdd5cebb6807166f0e489e91771cf7f2948e253e49ef9a5`

```rust
 9472: struct AbstractPointer {
 9473:     targets: BTreeSet<usize>,
 9474:     unknown: bool,
 9475:     may_be_null: bool,
 9476:     /// Card W43-LIBC-RESULT-ALIAS: whether this binding *is* its targets'
 9477:     /// identity or a non-owning pointer derived from them by a declared
 9478:     /// [`ResourceReturnMode::Derived`] result. Copied by every alias, joined by
 9479:     /// [`Derivation::join`].
 9480:     derivation: Derivation,
 9481: }
 9482: 
 9483: /// How a pointer binding relates to the resource identities it targets.
 9484: ///
 9485: /// Card W43-LIBC-RESULT-ALIAS. A `Derived` binding shares its base's targets,
 9486: /// so releasing the base invalidates it through the ordinary site status, but
 9487: /// it owns nothing: a release, escape, consumption, store or reallocation
 9488: /// through it is never applied to the base (see [`derived_custody_refusal`]),
 9489: /// its null test narrows only itself, and its nullness never flows back onto
 9490: /// the base's site.
 9491: #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
 9492: enum Derivation {
 9493:     /// The binding denotes its targets' identities themselves.
 9494:     #[default]
 9495:     Identity,
 9496:     /// The binding was produced by a declared derived result, or (card
 9497:     /// W43-INDEX-TRAVERSAL) by pointer arithmetic on a binding.
 9498:     Derived {
 9499:         extent: DerivedExtent,
 9500:         const_dropped: bool,
 9501:         /// Card W43-RESULT-ALIAS-RETURNED: whether some derivation step on
 9502:         /// the way here may return NULL for a non-null base (`strchr`). It is
 9503:         /// the *relative* nullness a derived-result summary states, kept apart
 9504:         /// from [`AbstractPointer::may_be_null`], which also carries the base's
 9505:         /// own nullness. It is joined by disjunction and never narrowed, so it
 9506:         /// can only over-state the summary's `nullable`. No custody rule reads
 9507:         /// it.
 9508:         nullable: bool,
 9509:         /// Card W43-INDEX-TRAVERSAL: what pointer arithmetic did to the binding.
 9510:         stepped: Stepped,
 9511:     },
 9512:     /// Different paths disagree. Treated as derived wherever that refuses, and
 9513:     /// never as derived where that would accuse.
 9514:     Mixed,
 9515: }
 9516: 
 9517: /// Card **W43-INDEX-TRAVERSAL**: what pointer arithmetic did to a derived
 9518: /// binding's position in its object.
 9519: ///
 9520: /// `No` is every derivation card W43-LIBC-RESULT-ALIAS made, and behaves
 9521: /// exactly as it did. The two stepped values describe a binding that went
 9522: /// through at least one `P++`, `P += E`, `P = Q ± E` or `*P++`:
 9523: ///
 9524: /// * `Forward` — every step moved it forward by a positive literal amount, so
 9525: ///   it lies strictly after the pointer it started from. Releasing it is an
 9526: ///   `invalid_release` candidate, as an interior result's release is.
 9527: /// * `Any` — it may lie anywhere in its object, the start included: some step
 9528: ///   went backward or by a variable amount, or a path joined it with the base
 9529: ///   identity itself (a loop's entry edge). Releasing it is refused, never
 9530: ///   accused, because it may be the value the allocator returned.
 9531: ///
 9532: /// The join keeps the two families apart: a stepped binding joined with an
 9533: /// unstepped derived one is `Mixed`, so the lattice stays a lattice (see
 9534: /// [`Derivation::join`]) and no existing derived result changes its answer.
 9535: #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
 9536: enum Stepped {
 9537:     #[default]
 9538:     No,
 9539:     Forward,
 9540:     Any,
 9541: }
 9542: 
 9543: impl Derivation {
 9544:     const fn is_identity(self) -> bool {
 9545:         matches!(self, Self::Identity)
 9546:     }
 9547: 
 9548:     /// Card **W43-INDEX-TRAVERSAL**: the derivation of `P ± E` over a binding
 9549:     /// of this derivation. Always interior; a mixed base stays mixed.
 9550:     fn step(self, forward: bool) -> Self {
 9551:         let movement = if forward {
 9552:             Stepped::Forward
 9553:         } else {
 9554:             Stepped::Any
 9555:         };
 9556:         match self {
 9557:             Self::Identity => Self::Derived {
 9558:                 extent: DerivedExtent::Interior,
 9559:                 const_dropped: false,
 9560:                 // Arithmetic on a non-null pointer never makes it null.
 9561:                 nullable: false,
 9562:                 stepped: movement,
 9563:             },
 9564:             Self::Derived {
 9565:                 const_dropped,
 9566:                 nullable,
 9567:                 stepped,
 9568:                 ..
 9569:             } => Self::Derived {
 9570:                 extent: DerivedExtent::Interior,
 9571:                 const_dropped,
 9572:                 nullable,
 9573:                 // A declared interior result stepped forward still lies after
 9574:                 // its base's start; anything that may be at the start stays so.
 9575:                 stepped: stepped.max(movement),
 9576:             },
 9577:             Self::Mixed => Self::Mixed,
 9578:         }
 9579:     }
 9580: 
 9581:     /// Join two paths' derivations. A definitely-null binding carries no
 9582:     /// relation, so it takes the other path's; otherwise equal derivations are
 9583:     /// kept, `const_dropped` and the relative `nullable` are joined by
 9584:     /// disjunction within one extent, and any other disagreement is `Mixed`.
 9585:     ///
 9586:     /// Card **W43-INDEX-TRAVERSAL** adds one join and keeps the rest: an
 9587:     /// identity and a *stepped* pointer into it join to a stepped pointer that
 9588:     /// may be anywhere in the object (`Any`), which is what a loop's header
 9589:     /// sees for `for (p = s; *p; p++)`. Two stepped pointers join to the
 9590:     /// larger movement. A stepped and an unstepped derivation, and an identity
 9591:     /// and an unstepped derivation, still join to `Mixed`.
 9592:     fn join(one: &AbstractPointer, other: &AbstractPointer) -> Self {
 9593:         let definitely_null = |pointer: &AbstractPointer| {
 9594:             pointer.targets.is_empty() && !pointer.unknown && pointer.may_be_null
 9595:         };
 9596:         if definitely_null(other) {
 9597:             return one.derivation;
 9598:         }
 9599:         if definitely_null(one) {
 9600:             return other.derivation;
 9601:         }
 9602:         Self::join_derivations(one.derivation, other.derivation)
 9603:     }
 9604: 
 9605:     fn join_derivations(one: Self, other: Self) -> Self {
 9606:         match (one, other) {
 9607:             (left, right) if left == right => left,
 9608:             (
 9609:                 Self::Identity,
 9610:                 Self::Derived {
 9611:                     extent: DerivedExtent::Interior,
 9612:                     const_dropped,
 9613:                     nullable,
 9614:                     stepped: Stepped::Forward | Stepped::Any,
 9615:                 },
 9616:             )
 9617:             | (
 9618:                 Self::Derived {
 9619:                     extent: DerivedExtent::Interior,
 9620:                     const_dropped,
 9621:                     nullable,
 9622:                     stepped: Stepped::Forward | Stepped::Any,
 9623:                 },
 9624:                 Self::Identity,
 9625:             ) => Self::Derived {
 9626:                 extent: DerivedExtent::Interior,
 9627:                 const_dropped,
 9628:                 // The identity path carries no relative nullness of its own.
 9629:                 nullable,
 9630:                 stepped: Stepped::Any,
 9631:             },
 9632:             (
 9633:                 Self::Derived {
 9634:                     extent,
 9635:                     const_dropped,
 9636:                     nullable,
 9637:                     stepped,
 9638:                 },
 9639:                 Self::Derived {
 9640:                     extent: other_extent,
 9641:                     const_dropped: other_dropped,
 9642:                     nullable: other_nullable,
 9643:                     stepped: other_stepped,
 9644:                 },
 9645:             ) if extent == other_extent
 9646:                 && ((stepped == Stepped::No) == (other_stepped == Stepped::No)) =>
 9647:             {
 9648:                 Self::Derived {
 9649:                     extent,
 9650:                     const_dropped: const_dropped || other_dropped,
 9651:                     nullable: nullable || other_nullable,
 9652:                     stepped: stepped.max(other_stepped),
 9653:                 }
 9654:             }
 9655:             _ => Self::Mixed,
 9656:         }
 9657:     }
 9658: }
 9659: 
 9660: #[derive(Clone, Debug, Default, PartialEq, Eq)]
 9661: struct AbstractState {
 9662:     pointers: BTreeMap<String, AbstractPointer>,
 9663:     resources: BTreeMap<usize, u8>,
 9664:     nullable_sites: BTreeSet<usize>,
 9665:     conditional_replacements: BTreeMap<usize, BTreeSet<usize>>,
 9666:     contained: BTreeMap<usize, BTreeSet<usize>>,
 9667:     /// O00 blocker B08: `owner site -> field name -> child sites` recorded by
 9668:     /// an admitted aggregate field store. Deliberately separate from
 9669:     /// `contained`: escaping the owner escapes its live linked children, while
 9670:     /// *releasing* the owner does not cascade, because a plain C field store
 9671:     /// carries no declared destructor obligation (premise P-B08-SHALLOW).
 9672:     field_links: BTreeMap<usize, BTreeMap<String, BTreeSet<usize>>>,
 9673:     /// Custody retained after a rejected effect; its residual is not a proven
 9674:     /// leak because the unmodeled action may have discharged it.
 9675:     unresolved_effect_sites: BTreeSet<usize>,
 9676:     widened: bool,
 9677:     /// Whether some instruction on *this* path was refused or reported an
 9678:     /// unresolved effect (O04). It is the per-path counterpart of the
 9679:     /// function-wide `Unknown` status: a path carrying it can support no
 9680:     /// custody claim at the exits it reaches. Joined by disjunction, so it is
 9681:     /// monotone and the work list still terminates.
 9682:     unmodeled: bool,
 9683:     /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the element ranges of every
 9684:     /// array site an element operation touched. A site absent here was never
 9685:     /// stored into, so all its elements are unfilled. Empty in every function
 9686:     /// that lowers no element operation, which keeps the join, and with it
 9687:     /// the work-list schedule, exactly as it was there (constraint 6).
 9688:     collections: BTreeMap<usize, crate::indexed_collection::Collection>,
 9689:     /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: `name < bound` facts a
 9690:     /// counted loop's body edge established, dropped when either name is
 9691:     /// written. Recorded only while some collection exists.
 9692:     index_bounds: BTreeMap<String, BTreeSet<crate::indexed_collection::Term>>,
 9693:     /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: bound names whose clamped
 9694:     /// value `max(0, name)` is zero — a counted loop left with its index still
 9695:     /// zero, so it ran no iteration. Dropped when the name is written.
 9696:     clamped_zero: BTreeSet<String>,
 9697:     /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the collection names
 9698:     /// (indices, bounds, counters) this path binds to zero, kept by the
 9699:     /// collection rules themselves so that a vacuous path cannot weaken them.
 9700:     index_zero: BTreeSet<String>,
 9701:     /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: no execution reaches here for
 9702:     /// the collection facts — a counted loop's body edge whose bound is zero
 9703:     /// when clamped, which its never-negative index cannot be below. The
 9704:     /// collection part of this state claims nothing and joins as the
 9705:     /// identity; the edge, and every other map on it, is kept.
 9706:     collection_vacuous: bool,
 9707:     /// Card **W45-FB-RETRY-LIFETIME**: the sites this path published into a
 9708:     /// caller's container ([`container_publication`]). Joined by intersection,
 9709:     /// so a site is published only when every arriving path published it.
 9710:     /// Empty in every function that calls no declared publication, which keeps
 9711:     /// the join, and with it the work-list schedule, as it was (constraint 6).
 9712:     published: BTreeSet<usize>,
 9713: }
 9714: 
 9715: /// The scalar facts one abstract state proves: a name bound to the null
 9716: /// constant (the lowering of `i = 0`) is zero, and a counted loop's body edge
 9717: /// bounds its index.
 9718: impl crate::indexed_collection::Scalars for AbstractState {
 9719:     fn is_zero(&self, name: &str) -> bool {
 9720:         self.index_zero.contains(name)
 9721:     }
 9722: 
 9723:     fn clamped_zero(&self, name: &str) -> bool {
 9724:         self.clamped_zero.contains(name)
 9725:     }
 9726: 
 9727:     fn upper_bounds(&self, name: &str) -> Vec<crate::indexed_collection::Term> {
 9728:         self.index_bounds
 9729:             .get(name)
 9730:             .map(|bounds| bounds.iter().cloned().collect())
 9731:             .unwrap_or_default()
 9732:     }
 9733: }
 9734: 
 9735: const MAX_ALIAS_TARGETS: usize = 8;
 9736: 
 9737: impl AbstractState {
 9738:     /// Join an incoming edge's state into this one; returns whether anything
 9739:     /// changed.
 9740:     ///
 9741:     /// Card **W43-INDEX-TRAVERSAL**: `scalars` are the function's declared
 9742:     /// scalar names ([`TraversalNames::scalars`]). A declared scalar's binding
 9743:     /// carries at most a branch fact and never a custody claim, so when one
 9744:     /// path has no binding for it and the other a binding with no target, the
 9745:     /// join has no fact either — the absent binding, which admits both branches
 9746:     /// of a later test and escapes nothing — rather than the unknown, possibly
 9747:     /// null *pointer* the generic arm makes of it. A scalar binding that does
 9748:     /// carry a target (a descriptor a contracted call produced) keeps the
 9749:     /// generic arm.
 9750:     fn join(&mut self, incoming: &Self, scalars: &BTreeSet<String>) -> bool {
 9751:         let before = self.clone();
 9752:         let variables = self
 9753:             .pointers
 9754:             .keys()
 9755:             .chain(incoming.pointers.keys())
 9756:             .cloned()
 9757:             .collect::<BTreeSet<_>>();
 9758:         for variable in variables {
 9759:             let one_sided_scalar_fact = scalars.contains(&variable)
 9760:                 && match (
 9761:                     self.pointers.get(&variable),
 9762:                     incoming.pointers.get(&variable),
 9763:                 ) {
 9764:                     (Some(pointer), None) | (None, Some(pointer)) => pointer.targets.is_empty(),
 9765:                     _ => false,
 9766:                 };
 9767:             if one_sided_scalar_fact {
 9768:                 self.pointers.remove(&variable);
 9769:                 continue;
 9770:             }
 9771:             match (
 9772:                 self.pointers.get(&variable).cloned(),
 9773:                 incoming.pointers.get(&variable),
 9774:             ) {
 9775:                 (Some(mut current), Some(other)) => {
 9776:                     current.derivation = Derivation::join(&current, other);
 9777:                     current.targets.extend(other.targets.iter().copied());
 9778:                     current.unknown |= other.unknown;
 9779:                     current.may_be_null |= other.may_be_null;
 9780:                     if current.targets.len() > MAX_ALIAS_TARGETS {
 9781:                         current.targets.clear();
 9782:                         current.unknown = true;
 9783:                         self.widened = true;
 9784:                     }
 9785:                     self.pointers.insert(variable, current);
 9786:                 }
 9787:                 (Some(mut current), None) => {
 9788:                     current.unknown = true;
 9789:                     current.may_be_null = true;
 9790:                     self.pointers.insert(variable, current);
```

<a id="q09"></a>

## Q09 — `crates/qcore-c-gateway/src/typed_source.rs:1–176`

File SHA-256: `60059f38261f2075f64f0829493f52f1a2d896986627a04be2f8b47c329afca4`

```rust
    1: //! Bounded typed C access paths shared by library source importers.
    2: //!
    3: //! These are source projections, NOT safety or custody certificates.
    4: //! Reads and calls yield fresh symbols; pointer aliases retain those symbols.
    5: //! Calls remain opaque (in particular, a recorded close call is not discharge).
    6: //! All branches are retained, including infeasible alternatives. Consumers must
    7: //! establish guards, pointer provenance, live/bounded storage, arithmetic, call
    8: //! contracts, and synchronization before interpreting a path as safe.
    9: 
   10: use std::collections::{BTreeMap, BTreeSet};
   11: 
   12: use serde::Serialize;
   13: use serde_json::Value;
   14: 
   15: use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
   16: use crate::typed_c_ast::{ast_declarations, children, function_ast, kind};
   17: 
   18: /// Default declarative acceptance rules used by the legacy typed-path importer.
   19: /// This is shared policy data, not source evidence or a safety certificate.
   20: pub const DEFAULT_SOURCE_RULES: &str = include_str!(
   21:     "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
   22: );
   23: 
   24: fn check_source_rule(name: &str, facts: &[(&str, i128)]) -> Result<(), String> {
   25:     qcore_contract::contract_observation::check_contract_facts(DEFAULT_SOURCE_RULES, name, facts)
   26: }
   27: 
   28: const MAX_PATHS: usize = 64;
   29: const MAX_OPERATIONS: usize = 512;
   30: 
   31: /// Typed symbolic values. A symbol refers to an earlier Read or Call in this
   32: /// same path. Integer casts and sizeof remain explicit, never host-evaluated.
   33: #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
   34: pub struct SourceValue {
   35:     pub c_type: String,
   36:     pub expression: SourceExpression,
   37: }
   38: 
   39: #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
   40: #[serde(tag = "kind", rename_all = "snake_case")]
   41: pub enum SourceExpression {
   42:     Function {
   43:         name: String,
   44:         signature: String,
   45:     },
   46:     Parameter {
   47:         index: usize,
   48:     },
   49:     LocalStorage {
   50:         local: usize,
   51:     },
   52:     /// Typed induction value, scoped to the enclosing ScanLoop iteration.
   53:     LoopIndex {
   54:         local: usize,
   55:     },
   56:     Integer {
   57:         value: String,
   58:     },
   59:     String {
   60:         value: String,
   61:     },
   62:     SizeOf {
   63:         argument_type: String,
   64:     },
   65:     /// Ordered initializer elements; layouts and bounds remain consumer obligations.
   66:     Aggregate {
   67:         elements: Vec<SourceValue>,
   68:     },
   69:     Symbol {
   70:         operation: usize,
   71:     },
   72:     Cast {
   73:         cast: String,
   74:         value: Box<SourceValue>,
   75:     },
   76:     Unary {
   77:         operator: String,
   78:         value: Box<SourceValue>,
   79:     },
   80:     Binary {
   81:         operator: String,
   82:         left: Box<SourceValue>,
   83:         right: Box<SourceValue>,
   84:     },
   85:     Field {
   86:         base: Box<SourceValue>,
   87:         name: String,
   88:     },
   89:     Index {
   90:         base: Box<SourceValue>,
   91:         index: Box<SourceValue>,
   92:     },
   93: }
   94: 
   95: #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
   96: #[serde(tag = "kind", rename_all = "snake_case")]
   97: pub enum SourceOperation {
   98:     /// Repeat header, then match test, then store increment into local. A true
   99:     /// match executes early_return and exits the FUNCTION; a false header continues outside
  100:     /// the loop. Bounds, arithmetic and termination are consumer obligations.
  101:     ScanLoop {
  102:         local: usize,
  103:         initial: SourceValue,
  104:         header: Box<SourceRegion>,
  105:         matches: Box<SourceRegion>,
  106:         early_return: Box<SourceRegion>,
  107:         increment: SourceValue,
  108:     },
  109:     /// Storage type is carried by place; this operation yields that type.
  110:     Read {
  111:         place: SourceValue,
  112:     },
  113:     Write {
  114:         place: SourceValue,
  115:         value: SourceValue,
  116:     },
  117:     /// Stack values are retained for address-taking (e.g. send's &val).
  118:     LocalWrite {
  119:         local: usize,
  120:         value: SourceValue,
  121:     },
  122:     LocalStart {
  123:         local: usize,
  124:         c_type: String,
  125:     },
  126:     /// Signature is the source declaration's type, not an assumed API contract.
  127:     Call {
  128:         function: String,
  129:         signature: String,
  130:         arguments: Vec<SourceValue>,
  131:         result_type: String,
  132:     },
  133:     Guard {
  134:         condition: SourceValue,
  135:         taken: bool,
  136:     },
  137:     LocalEnd {
  138:         local: usize,
  139:     },
  140:     ReturnValue {
  141:         value: SourceValue,
  142:     },
  143:     Return,
  144:     /// A source-declared nonreturning call ended this path. This is not teardown.
  145:     Terminate,
  146: }
  147: 
  148: /// Symbol indices continue from the enclosing path through the three regions
  149: /// of one iteration. Iteration producers are never available after loop exit.
  150: #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
  151: pub struct SourceRegion {
  152:     pub operations: Vec<SourceOperation>,
  153:     pub result: SourceValue,
  154: }
  155: 
  156: #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
  157: pub struct SourcePath {
  158:     pub operations: Vec<SourceOperation>,
  159: }
  160: 
  161: /// Construction is private: every definition node must pass the importer.
  162: /// A serialized path report cannot be deserialized into source authority.
  163: ///
  164: /// ```compile_fail
  165: /// use qcore_c_gateway::typed_source::SourceSummary;
  166: /// let forged: SourceSummary = serde_json::from_str("{}").unwrap();
  167: /// ```
  168: /// Nor can a caller manufacture one from mutable path records.
  169: ///
  170: /// ```compile_fail
  171: /// use qcore_c_gateway::typed_source::SourceSummary;
  172: /// let forged = SourceSummary {
  173: ///     function: "f".into(), source_revision: "unverified".into(),
  174: ///     signature: "void ()".into(), paths: vec![],
  175: ///     profile_assumptions: Default::default(),
  176: /// };
```

<a id="q10"></a>

## Q10 — `crates/qcore-c-gateway/src/typed_source.rs:469–579`

File SHA-256: `60059f38261f2075f64f0829493f52f1a2d896986627a04be2f8b47c329afca4`

```rust
  469:         }
  470:     }
  471:     let mut names = BTreeSet::new();
  472:     references(ast, &mut names);
  473:     let mut constants = BTreeMap::new();
  474:     for name in names {
  475:         let declarations = ast_declarations(source, unit, &name)?;
  476:         let exact: Vec<_> = declarations
  477:             .iter()
  478:             .filter(|node| kind(node) == "EnumConstantDecl" && node["name"] == name)
  479:             .collect();
  480:         let [decl] = exact.as_slice() else {
  481:             return Err("enum constant requires one unambiguous declaration".into());
  482:         };
  483:         let [initial] = children(decl) else {
  484:             return Err("enum constant requires an explicit evaluated initializer".into());
  485:         };
  486:         check_source_rule(
  487:             "source_enum_value",
  488:             &[
  489:                 (
  490:                     "evaluated_constant",
  491:                     i128::from(kind(initial) == "ConstantExpr"),
  492:                 ),
  493:                 (
  494:                     "type_matches",
  495:                     i128::from(c_type(initial)? == c_type(decl)?),
  496:                 ),
  497:             ],
  498:         )?;
  499:         let integer = initial["value"]
  500:             .as_str()
  501:             .ok_or("missing evaluated enum value")?;
  502:         integer
  503:             .parse::<i128>()
  504:             .map_err(|_| "enum value outside integer profile")?;
  505:         constants.insert(
  506:             name,
  507:             value(
  508:                 c_type(decl)?,
  509:                 SourceExpression::Integer {
  510:                     value: integer.into(),
  511:                 },
  512:             ),
  513:         );
  514:     }
  515:     Ok(constants)
  516: }
  517: 
  518: fn c_type(node: &Value) -> Result<String, String> {
  519:     let ty = node["type"]["desugaredQualType"]
  520:         .as_str()
  521:         .or_else(|| node["type"]["qualType"].as_str())
  522:         .ok_or("missing C type")?;
  523:     if ty.contains("volatile") || ty.contains("_Atomic") || ty.contains("restrict") {
  524:         return Err("qualified access requires additional synchronization/alias evidence".into());
  525:     }
  526:     Ok(ty.into())
  527: }
  528: 
  529: fn value(ty: String, expression: SourceExpression) -> SourceValue {
  530:     SourceValue {
  531:         c_type: ty,
  532:         expression,
  533:     }
  534: }
  535: 
  536: #[derive(Clone)]
  537: enum Expr {
  538:     Local(String, String),
  539:     Literal(SourceValue),
  540:     Cast(String, String, Box<Self>),
  541:     Unary(String, String, Box<Self>),
  542:     Binary(String, String, Box<Self>, Box<Self>),
  543:     Field(String, Box<Self>, String),
  544:     Index(String, Box<Self>, Box<Self>),
  545:     Read(String, Box<Self>),
  546:     Call(String, String, String, Vec<Self>),
  547:     Aggregate(String, Vec<Self>),
  548: }
  549: 
  550: #[derive(Clone)]
  551: enum Stmt {
  552:     Block(Vec<Self>),
  553:     Declare(String, String, Option<Expr>),
  554:     Assign(Expr, Expr),
  555:     OrAssign(Expr, Expr),
  556:     Increment(Expr, String, String),
  557:     Call(Expr),
  558:     NoReturnCall(Expr),
  559:     DoOnce(Box<Self>, Expr),
  560:     ScanLoop(Box<Self>, String, String, Expr, Expr, Expr),
  561:     If(Expr, Box<Self>, Option<Box<Self>>),
  562:     Return(Option<Expr>),
  563: }
  564: 
  565: struct Importer<'a> {
  566:     fields: &'a BTreeMap<(String, String), String>,
  567:     constants: &'a BTreeMap<String, SourceValue>,
  568:     allowed_calls: Option<&'a [&'a str]>,
  569:     sizeof_types: &'a [&'a str],
  570:     arrays: &'a BTreeSet<String>,
  571:     locals: BTreeSet<String>,
  572:     nodes: usize,
  573: }
  574: 
  575: impl Importer<'_> {
  576:     fn budget(&mut self, depth: usize) -> Result<(), String> {
  577:         self.nodes += 1;
  578:         if depth > 64 || self.nodes > 2048 {
  579:             return Err("callback exceeds bounded source import size/depth".into());
```

<a id="q11"></a>

## Q11 — `crates/qcore-c-gateway/src/typed_source.rs:582–796`

File SHA-256: `60059f38261f2075f64f0829493f52f1a2d896986627a04be2f8b47c329afca4`

```rust
  582:     }
  583: 
  584:     fn expression(&mut self, node: &Value, depth: usize) -> Result<Expr, String> {
  585:         self.budget(depth)?;
  586:         let ty = c_type(node)?;
  587:         let inner = children(node);
  588:         match (kind(node), inner) {
  589:             ("DeclRefExpr", []) if node["referencedDecl"]["kind"] == "EnumConstantDecl" => {
  590:                 let constant = node["referencedDecl"]["name"]
  591:                     .as_str()
  592:                     .and_then(|name| self.constants.get(name))
  593:                     .filter(|constant| constant.c_type == ty)
  594:                     .ok_or("unresolved or mismatched enum constant")?;
  595:                 Ok(Expr::Literal(constant.clone()))
  596:             }
  597:             ("InitListExpr", elements) if ty == "struct timeval" => {
  598:                 if elements.len() != 2
  599:                     || !self
  600:                         .fields
  601:                         .contains_key(&("struct timeval *".into(), "tv_sec".into()))
  602:                     || !self
  603:                         .fields
  604:                         .contains_key(&("struct timeval *".into(), "tv_usec".into()))
  605:                 {
  606:                     return Err("record initializer outside fixed timeval profile".into());
  607:                 }
  608:                 Ok(Expr::Aggregate(
  609:                     ty,
  610:                     elements
  611:                         .iter()
  612:                         .map(|element| self.expression(element, depth + 1))
  613:                         .collect::<Result<_, _>>()?,
  614:                 ))
  615:             }
  616:             ("DeclRefExpr", [])
  617:                 if matches!(
  618:                     node["referencedDecl"]["kind"].as_str(),
  619:                     Some("VarDecl" | "ParmVarDecl")
  620:                 ) =>
  621:             {
  622:                 let id = node["referencedDecl"]["id"]
  623:                     .as_str()
  624:                     .ok_or("missing local ID")?;
  625:                 if !self.locals.contains(id) {
  626:                     return Err("global or unresolved local reference".into());
  627:                 }
  628:                 Ok(Expr::Local(id.into(), ty))
  629:             }
  630:             ("DeclRefExpr", []) if node["referencedDecl"]["kind"] == "FunctionDecl" => {
  631:                 Ok(Expr::Literal(value(
  632:                     ty.clone(),
  633:                     SourceExpression::Function {
  634:                         name: node["referencedDecl"]["name"]
  635:                             .as_str()
  636:                             .ok_or("missing function identity")?
  637:                             .into(),
  638:                         signature: ty,
  639:                     },
  640:                 )))
  641:             }
  642:             ("IntegerLiteral", []) | ("StringLiteral", []) => {
  643:                 let literal = node["value"].as_str().ok_or("missing literal")?.to_owned();
  644:                 Ok(Expr::Literal(value(
  645:                     ty,
  646:                     if kind(node) == "IntegerLiteral" {
  647:                         SourceExpression::Integer { value: literal }
  648:                     } else {
  649:                         SourceExpression::String { value: literal }
  650:                     },
  651:                 )))
  652:             }
  653:             ("UnaryExprOrTypeTraitExpr", []) if node["name"] == "sizeof" => {
  654:                 let argument_type = node["argType"]["desugaredQualType"]
  655:                     .as_str()
  656:                     .or_else(|| node["argType"]["qualType"].as_str())
  657:                     .ok_or("unsupported sizeof operand")?;
  658:                 // Reject variably modified sizeof, whose operand can execute.
  659:                 if !self.sizeof_types.contains(&argument_type) {
  660:                     return Err("sizeof outside fixed scalar type profile".into());
  661:                 }
  662:                 Ok(Expr::Literal(value(
  663:                     ty,
  664:                     SourceExpression::SizeOf {
  665:                         argument_type: argument_type.into(),
  666:                     },
  667:                 )))
  668:             }
  669:             ("ParenExpr", [child]) => self.expression(child, depth + 1),
  670:             ("PredefinedExpr", [child]) if kind(child) == "StringLiteral" => {
  671:                 self.expression(child, depth + 1)
  672:             }
  673:             ("ImplicitCastExpr", [child]) if node["castKind"] == "LValueToRValue" => {
  674:                 Ok(Expr::Read(ty, Box::new(self.expression(child, depth + 1)?)))
  675:             }
  676:             ("ImplicitCastExpr" | "CStyleCastExpr", [child])
  677:                 if matches!(
  678:                     node["castKind"].as_str(),
  679:                     Some(
  680:                         "BitCast"
  681:                             | "IntegralCast"
  682:                             | "NoOp"
  683:                             | "ArrayToPointerDecay"
  684:                             | "NullToPointer"
  685:                             | "FunctionToPointerDecay"
  686:                     )
  687:                 ) =>
  688:             {
  689:                 Ok(Expr::Cast(
  690:                     ty,
  691:                     node["castKind"].as_str().unwrap().into(),
  692:                     Box::new(self.expression(child, depth + 1)?),
  693:                 ))
  694:             }
  695:             ("MemberExpr", [base]) if node["isArrow"] == true => {
  696:                 let name = node["name"].as_str().ok_or("missing field name")?;
  697:                 if self.fields.get(&(c_type(base)?, name.into())) != Some(&ty) {
  698:                     return Err("field is not a direct ordinary field of a selected record".into());
  699:                 }
  700:                 Ok(Expr::Field(
  701:                     ty,
  702:                     Box::new(self.expression(base, depth + 1)?),
  703:                     name.into(),
  704:                 ))
  705:             }
  706:             ("MemberExpr", [base]) if node["isArrow"] == false => {
  707:                 let name = node["name"].as_str().ok_or("missing field name")?;
  708:                 let pointer_type = format!("{} *", c_type(base)?);
  709:                 if self.fields.get(&(pointer_type.clone(), name.into())) != Some(&ty) {
  710:                     return Err("dot field outside selected ordinary record layout".into());
  711:                 }
  712:                 Ok(Expr::Field(
  713:                     ty,
  714:                     Box::new(Expr::Unary(
  715:                         pointer_type,
  716:                         "&".into(),
  717:                         Box::new(self.expression(base, depth + 1)?),
  718:                     )),
  719:                     name.into(),
  720:                 ))
  721:             }
  722:             ("ArraySubscriptExpr", [base, index]) => Ok(Expr::Index(
  723:                 ty,
  724:                 Box::new(self.expression(base, depth + 1)?),
  725:                 Box::new(self.expression(index, depth + 1)?),
  726:             )),
  727:             ("UnaryOperator", [child])
  728:                 if matches!(node["opcode"].as_str(), Some("&" | "*" | "-" | "!")) =>
  729:             {
  730:                 Ok(Expr::Unary(
  731:                     ty,
  732:                     node["opcode"].as_str().unwrap().into(),
  733:                     Box::new(self.expression(child, depth + 1)?),
  734:                 ))
  735:             }
  736:             ("BinaryOperator", [left, right])
  737:                 if matches!(
  738:                     node["opcode"].as_str(),
  739:                     Some("&" | "|" | "==" | "!=" | "<" | ">" | "+" | "*" | "/" | "%" | "&&" | "||")
  740:                 ) =>
  741:             {
  742:                 Ok(Expr::Binary(
  743:                     ty,
  744:                     node["opcode"].as_str().unwrap().into(),
  745:                     Box::new(self.expression(left, depth + 1)?),
  746:                     Box::new(self.expression(right, depth + 1)?),
  747:                 ))
  748:             }
  749:             ("CallExpr", [callee, arguments @ ..]) => {
  750:                 let [reference] = children(callee) else {
  751:                     return Err("indirect or unmodeled call".into());
  752:                 };
  753:                 if kind(callee) != "ImplicitCastExpr"
  754:                     || !matches!(
  755:                         callee["castKind"].as_str(),
  756:                         Some("FunctionToPointerDecay" | "BuiltinFnToFnPtr")
  757:                     )
  758:                     || kind(reference) != "DeclRefExpr"
  759:                     || reference["referencedDecl"]["kind"] != "FunctionDecl"
  760:                 {
  761:                     return Err("indirect or unmodeled call".into());
  762:                 }
  763:                 let name = reference["referencedDecl"]["name"]
  764:                     .as_str()
  765:                     .ok_or("missing call name")?;
  766:                 // This list bounds syntax only. It does not assume these calls
  767:                 // succeed or have a particular custody/destruction effect.
  768:                 if self
  769:                     .allowed_calls
  770:                     .is_some_and(|calls| !calls.contains(&name))
  771:                 {
  772:                     return Err(format!(
  773:                         "call {name} is outside the callback source profile"
  774:                     ));
  775:                 }
  776:                 Ok(Expr::Call(
  777:                     ty,
  778:                     name.into(),
  779:                     c_type(reference)?,
  780:                     arguments
  781:                         .iter()
  782:                         .map(|arg| self.expression(arg, depth + 1))
  783:                         .collect::<Result<_, _>>()?,
  784:                 ))
  785:             }
  786:             _ => Err(format!("unmodeled callback expression {}", kind(node))),
  787:         }
  788:     }
  789: 
  790:     fn statement(&mut self, node: &Value, depth: usize) -> Result<Stmt, String> {
  791:         self.budget(depth)?;
  792:         match (kind(node), children(node)) {
  793:             ("CompoundStmt", statements) => Ok(Stmt::Block(
  794:                 statements
  795:                     .iter()
  796:                     .map(|stmt| self.statement(stmt, depth + 1))
```

<a id="q12"></a>

## Q12 — `crates/qcore-c-gateway/src/typed_source.rs:798–960`

File SHA-256: `60059f38261f2075f64f0829493f52f1a2d896986627a04be2f8b47c329afca4`

```rust
  798:             )),
  799:             ("NullStmt", []) => Ok(Stmt::Block(vec![])),
  800:             ("ForStmt", [initial, condition_variable, condition, increment, body]) => {
  801:                 if !kind(condition_variable).is_empty() {
  802:                     return Err("scan loop has a condition variable".into());
  803:                 }
  804:                 let [index] = children(increment) else {
  805:                     return Err("scan loop requires a local increment".into());
  806:                 };
  807:                 let Expr::Local(id, ty) = self.expression(index, depth + 1)? else {
  808:                     return Err("scan index is not local".into());
  809:                 };
  810:                 if kind(increment) != "UnaryOperator"
  811:                     || increment["opcode"] != "++"
  812:                     || !matches!(
  813:                         ty.as_str(),
  814:                         "int" | "unsigned int" | "unsigned long" | "unsigned long long"
  815:                     )
  816:                 {
  817:                     return Err("scan loop requires a supported integer increment".into());
  818:                 }
  819:                 let initial = self.statement(initial, depth + 1)?;
  820:                 if !matches!(&initial, Stmt::Assign(Expr::Local(target, _), _) if target == &id) {
  821:                     return Err("scan initializer must assign its induction local".into());
  822:                 }
  823:                 let condition = self.expression(condition, depth + 1)?;
  824:                 let body = self.statement(body, depth + 1)?;
  825:                 fn single(stmt: &Stmt) -> &Stmt {
  826:                     if let Stmt::Block(statements) = stmt
  827:                         && let [only] = statements.as_slice()
  828:                     {
  829:                         single(only)
  830:                     } else {
  831:                         stmt
  832:                     }
  833:                 }
  834:                 let Stmt::If(matches, yes, None) = single(&body) else {
  835:                     return Err("scan body requires one conditional early return".into());
  836:                 };
  837:                 let Stmt::Return(Some(result)) = single(yes) else {
  838:                     return Err("scan match must return from the function".into());
  839:                 };
  840:                 check_source_rule(
  841:                     "source_scan_effects",
  842:                     &[
  843:                         ("header_calls", i128::from(effects(&condition).1)),
  844:                         ("match_calls", i128::from(effects(matches).1)),
  845:                         ("return_calls", i128::from(effects(result).1)),
  846:                     ],
  847:                 )?;
  848:                 Ok(Stmt::ScanLoop(
  849:                     Box::new(initial),
  850:                     id,
  851:                     ty,
  852:                     condition,
  853:                     matches.clone(),
  854:                     result.clone(),
  855:                 ))
  856:             }
  857:             ("DoStmt", [body, condition]) => {
  858:                 // Macro-style single execution only. Parse the entire body;
  859:                 // break/continue and nonliteral conditions still fail closed.
  860:                 let mut literal = condition;
  861:                 while kind(literal) == "ParenExpr" && children(literal).len() == 1 {
  862:                     literal = &children(literal)[0];
  863:                 }
  864:                 if kind(literal) != "IntegerLiteral" || literal["value"] != "0" {
  865:                     return Err("do loop requires a literal zero condition".into());
  866:                 }
  867:                 Ok(Stmt::DoOnce(
  868:                     Box::new(self.statement(body, depth + 1)?),
  869:                     self.expression(condition, depth + 1)?,
  870:                 ))
  871:             }
  872:             ("DeclStmt", [decl])
  873:                 if kind(decl) == "VarDecl" && decl.get("storageClass").is_none() =>
  874:             {
  875:                 let id = decl["id"]
  876:                     .as_str()
  877:                     .ok_or("missing declaration ID")?
  878:                     .to_owned();
  879:                 let ty = c_type(decl)?;
  880:                 // Fixed storage only: no VLA, implicit aggregate initialization,
  881:                 // or destructor attributes. Array cells are explicit memory reads.
  882:                 if !(ty.ends_with('*')
  883:                     || self.arrays.contains(&ty)
  884:                     || matches!(
  885:                         ty.as_str(),
  886:                         "int"
  887:                             | "short"
  888:                             | "unsigned int"
  889:                             | "unsigned long"
  890:                             | "unsigned long long"
  891:                             | "long"
  892:                             | "int[2]"
  893:                             | "struct timeval"
  894:                     ))
  895:                 {
  896:                     return Err("local outside scalar/pointer callback profile".into());
  897:                 }
  898:                 let initial = match children(decl) {
  899:                     [] => None,
  900:                     [expr] => Some(self.expression(expr, depth + 1)?),
  901:                     _ => return Err("unmodeled local initialization/attributes".into()),
  902:                 };
  903:                 self.locals.insert(id.clone());
  904:                 Ok(Stmt::Declare(id, ty, initial))
  905:             }
  906:             ("BinaryOperator", [left, right]) if node["opcode"] == "=" => Ok(Stmt::Assign(
  907:                 self.expression(left, depth + 1)?,
  908:                 self.expression(right, depth + 1)?,
  909:             )),
  910:             ("CompoundAssignOperator", [left, right]) if node["opcode"] == "|=" => {
  911:                 if c_type(left)? != "int" || c_type(right)? != "int" {
  912:                     return Err("compound assignment requires int operands".into());
  913:                 }
  914:                 Ok(Stmt::OrAssign(
  915:                     self.expression(left, depth + 1)?,
  916:                     self.expression(right, depth + 1)?,
  917:                 ))
  918:             }
  919:             ("UnaryOperator", [place])
  920:                 if matches!(node["opcode"].as_str(), Some("++" | "--"))
  921:                     && matches!(
  922:                         c_type(node)?.as_str(),
  923:                         "int" | "unsigned int" | "unsigned long" | "unsigned long long"
  924:                     ) =>
  925:             {
  926:                 let operator = if node["opcode"] == "++" { "+" } else { "-" };
  927:                 Ok(Stmt::Increment(
  928:                     self.expression(place, depth + 1)?,
  929:                     operator.into(),
  930:                     c_type(node)?,
  931:                 ))
  932:             }
  933:             ("IfStmt", [condition, yes]) if node["hasElse"] != true => Ok(Stmt::If(
  934:                 self.expression(condition, depth + 1)?,
  935:                 Box::new(self.statement(yes, depth + 1)?),
  936:                 None,
  937:             )),
  938:             ("IfStmt", [condition, yes, no]) if node["hasElse"] == true => Ok(Stmt::If(
  939:                 self.expression(condition, depth + 1)?,
  940:                 Box::new(self.statement(yes, depth + 1)?),
  941:                 Some(Box::new(self.statement(no, depth + 1)?)),
  942:             )),
  943:             ("CallExpr", _) => {
  944:                 let expr = self.expression(node, depth + 1)?;
  945:                 if matches!(&expr, Expr::Call(_, _, signature, _) if signature.ends_with(" __attribute__((noreturn))"))
  946:                 {
  947:                     Ok(Stmt::NoReturnCall(expr))
  948:                 } else {
  949:                     Ok(Stmt::Call(expr))
  950:                 }
  951:             }
  952:             ("ReturnStmt", []) => Ok(Stmt::Return(None)),
  953:             ("ReturnStmt", [result]) => Ok(Stmt::Return(Some(self.expression(result, depth + 1)?))),
  954:             _ => Err(format!("unmodeled callback statement {}", kind(node))),
  955:         }
  956:     }
  957: }
  958: 
  959: #[derive(Clone, Default)]
  960: struct Path {
```

<a id="q13"></a>

## Q13 — `crates/qcore-c-gateway/src/typed_source.rs:994–1076`

File SHA-256: `60059f38261f2075f64f0829493f52f1a2d896986627a04be2f8b47c329afca4`

```rust
  994:         let result = match expression {
  995:             Expr::Local(id, ty) => {
  996:                 let local = self.locals.get(id).ok_or("local outside its scope")?;
  997:                 value(
  998:                     ty.clone(),
  999:                     SourceExpression::LocalStorage { local: local.0 },
 1000:                 )
 1001:             }
 1002:             Expr::Literal(value) => value.clone(),
 1003:             Expr::Aggregate(ty, elements) => {
 1004:                 reject_unsequenced(&elements.iter().collect::<Vec<_>>())?;
 1005:                 value(
 1006:                     ty.clone(),
 1007:                     SourceExpression::Aggregate {
 1008:                         elements: elements
 1009:                             .iter()
 1010:                             .map(|element| self.evaluate(element))
 1011:                             .collect::<Result<_, _>>()?,
 1012:                     },
 1013:                 )
 1014:             }
 1015:             Expr::Read(ty, place) => self.read(place, ty)?,
 1016:             Expr::Cast(ty, cast, child) => value(
 1017:                 ty.clone(),
 1018:                 SourceExpression::Cast {
 1019:                     cast: cast.clone(),
 1020:                     value: Box::new(self.evaluate(child)?),
 1021:                 },
 1022:             ),
 1023:             Expr::Unary(ty, operator, child) => {
 1024:                 if operator == "&"
 1025:                     && let Expr::Local(id, _) = child.as_ref()
 1026:                 {
 1027:                     self.addressed.insert(id.clone());
 1028:                 }
 1029:                 value(
 1030:                     ty.clone(),
 1031:                     SourceExpression::Unary {
 1032:                         operator: operator.clone(),
 1033:                         value: Box::new(self.evaluate(child)?),
 1034:                     },
 1035:                 )
 1036:             }
 1037:             Expr::Binary(_, operator, _, _) if matches!(operator.as_str(), "&&" | "||") => {
 1038:                 return Err("short-circuit values require a supported branch condition".into());
 1039:             }
 1040:             Expr::Binary(ty, operator, left, right) => {
 1041:                 reject_unsequenced(&[left, right])?;
 1042:                 value(
 1043:                     ty.clone(),
 1044:                     SourceExpression::Binary {
 1045:                         operator: operator.clone(),
 1046:                         left: Box::new(self.evaluate(left)?),
 1047:                         right: Box::new(self.evaluate(right)?),
 1048:                     },
 1049:                 )
 1050:             }
 1051:             Expr::Field(ty, base, name) => value(
 1052:                 ty.clone(),
 1053:                 SourceExpression::Field {
 1054:                     base: Box::new(self.evaluate(base)?),
 1055:                     name: name.clone(),
 1056:                 },
 1057:             ),
 1058:             Expr::Index(ty, base, index) => {
 1059:                 reject_unsequenced(&[base, index])?;
 1060:                 value(
 1061:                     ty.clone(),
 1062:                     SourceExpression::Index {
 1063:                         base: Box::new(self.evaluate(base)?),
 1064:                         index: Box::new(self.evaluate(index)?),
 1065:                     },
 1066:                 )
 1067:             }
 1068:             Expr::Call(ty, function, signature, arguments) => {
 1069:                 if signature.contains("noreturn") {
 1070:                     return Err("nonreturning call requires a terminal statement".into());
 1071:                 }
 1072:                 self.call(ty, function, signature, arguments)?
 1073:             }
 1074:         };
 1075:         check_value_size(&result)?;
 1076:         Ok(result)
```

<a id="q14"></a>

## Q14 — `crates/qcore-c-gateway/src/typed_source.rs:1315–1410`

File SHA-256: `60059f38261f2075f64f0829493f52f1a2d896986627a04be2f8b47c329afca4`

```rust
 1315: fn effects(expr: &Expr) -> (bool, bool) {
 1316:     let combine = |a: (bool, bool), b: (bool, bool)| (a.0 || b.0, a.1 || b.1);
 1317:     match expr {
 1318:         Expr::Local(..) | Expr::Literal(..) => (false, false),
 1319:         Expr::Read(_, child) => combine((true, false), effects(child)),
 1320:         Expr::Cast(_, _, child) | Expr::Unary(_, _, child) | Expr::Field(_, child, _) => {
 1321:             effects(child)
 1322:         }
 1323:         Expr::Binary(_, _, left, right) | Expr::Index(_, left, right) => {
 1324:             combine(effects(left), effects(right))
 1325:         }
 1326:         Expr::Call(..) => (true, true),
 1327:         Expr::Aggregate(_, elements) => elements.iter().fold((false, false), |prior, element| {
 1328:             combine(prior, effects(element))
 1329:         }),
 1330:     }
 1331: }
 1332: 
 1333: fn reject_unsequenced(expressions: &[&Expr]) -> Result<(), String> {
 1334:     let effects: Vec<_> = expressions.iter().map(|expr| effects(expr)).collect();
 1335:     if effects.iter().any(|effect| effect.1) && effects.iter().filter(|effect| effect.0).count() > 1
 1336:     {
 1337:         Err("unsequenced call/read operands require an explicit ordering model".into())
 1338:     } else {
 1339:         Ok(())
 1340:     }
 1341: }
 1342: 
 1343: // Preserve C sequencing without adding eager logical operators to SourceValue.
 1344: // Only branch conditions receive this expansion; logical values in assignments,
 1345: // arguments, comparisons or scan regions remain Unknown. Cap every intermediate
 1346: // frontier as well as completed function paths.
 1347: fn branch_condition(condition: &Expr, path: Path, taken: bool) -> Result<Vec<Path>, String> {
 1348:     fn contains_logical(expr: &Expr) -> bool {
 1349:         match expr {
 1350:             Expr::Binary(_, op, _, _) if matches!(op.as_str(), "&&" | "||") => true,
 1351:             Expr::Unary(_, op, child) if op == "!" => contains_logical(child),
 1352:             _ => false,
 1353:         }
 1354:     }
 1355:     match condition {
 1356:         Expr::Unary(_, op, child) if op == "!" && contains_logical(child) => {
 1357:             branch_condition(child, path, !taken)
 1358:         }
 1359:         Expr::Binary(_, op, left, right) if matches!(op.as_str(), "&&" | "||") => {
 1360:             let is_and = op == "&&";
 1361:             let mut output = Vec::new();
 1362:             // true-first ordering matches existing conditional path ordering.
 1363:             for left_taken in [true, false] {
 1364:                 let evaluate_right = left_taken == is_and;
 1365:                 if !evaluate_right && taken != left_taken {
 1366:                     continue;
 1367:                 }
 1368:                 for left_path in branch_condition(left, path.clone(), left_taken)? {
 1369:                     if evaluate_right {
 1370:                         output.extend(branch_condition(right, left_path, taken)?);
 1371:                     } else {
 1372:                         output.push(left_path);
 1373:                     }
 1374:                     if output.len() > MAX_PATHS {
 1375:                         return Err("short-circuit path limit exceeded".into());
 1376:                     }
 1377:                 }
 1378:             }
 1379:             Ok(output)
 1380:         }
 1381:         _ => {
 1382:             let mut path = path;
 1383:             let condition = path.evaluate(condition)?;
 1384:             path.push(SourceOperation::Guard { condition, taken })?;
 1385:             Ok(vec![path])
 1386:         }
 1387:     }
 1388: }
 1389: 
 1390: fn execute(statement: &Stmt, paths: Vec<Path>) -> Result<Vec<Path>, String> {
 1391:     if let Stmt::Block(statements) = statement {
 1392:         let declared: Vec<_> = statements
 1393:             .iter()
 1394:             .filter_map(|statement| {
 1395:                 if let Stmt::Declare(id, _, _) = statement {
 1396:                     Some(id.clone())
 1397:                 } else {
 1398:                     None
 1399:                 }
 1400:             })
 1401:             .collect();
 1402:         let mut paths = paths;
 1403:         for statement in statements {
 1404:             paths = execute(statement, paths)?;
 1405:         }
 1406:         for path in &mut paths {
 1407:             // Remove block storage even on an early return. The final Return
 1408:             // operation is emitted after these lexical lifetime exits.
 1409:             for id in declared.iter().rev() {
 1410:                 if let Some((local, _, _)) = path.locals.remove(id) {
```

<a id="q15"></a>

## Q15 — `crates/qcore-c-gateway/src/typed_source.rs:1595–1723`

File SHA-256: `60059f38261f2075f64f0829493f52f1a2d896986627a04be2f8b47c329afca4`

```rust
 1595: fn derive(
 1596:     ast: &Value,
 1597:     function: &str,
 1598:     revision: &str,
 1599:     fields: &BTreeMap<(String, String), String>,
 1600:     constants: &BTreeMap<String, SourceValue>,
 1601:     arrays: &BTreeSet<String>,
 1602:     profile: &SourceImportProfile<'_>,
 1603: ) -> Result<SourceSummary, String> {
 1604:     let expected_signature = profile
 1605:         .signature_constraints
 1606:         .iter()
 1607:         .find(|(name, _, _)| *name == function);
 1608:     let signature = c_type(ast)?;
 1609:     let return_type = signature
 1610:         .split('(')
 1611:         .next()
 1612:         .ok_or("missing return type")?
 1613:         .trim();
 1614:     if ast["variadic"] == true
 1615:         || expected_signature.is_some_and(|(_, expected, _)| return_type != *expected)
 1616:     {
 1617:         return Err("function signature is outside the selected Unix profile".into());
 1618:     }
 1619:     if !matches!(
 1620:         return_type,
 1621:         "void"
 1622:             | "void *"
 1623:             | "int"
 1624:             | "char *"
 1625:             | "unsigned int"
 1626:             | "unsigned long"
 1627:             | "unsigned long long"
 1628:     ) {
 1629:         return Err("wrapper return type outside bounded profile".into());
 1630:     }
 1631:     let Some((body, parameters)) = children(ast).split_last() else {
 1632:         return Err("missing function body".into());
 1633:     };
 1634:     if kind(body) != "CompoundStmt"
 1635:         || expected_signature.is_some_and(|(_, _, types)| parameters.len() != types.len())
 1636:     {
 1637:         return Err("unmodeled function parameters/attributes".into());
 1638:     }
 1639:     let mut importer = Importer {
 1640:         fields,
 1641:         constants,
 1642:         allowed_calls: expected_signature.map(|_| profile.restricted_calls),
 1643:         sizeof_types: profile.sizeof_types,
 1644:         arrays,
 1645:         locals: BTreeSet::new(),
 1646:         nodes: 0,
 1647:     };
 1648:     let mut path = Path {
 1649:         next_local: parameters.len(),
 1650:         ..Path::default()
 1651:     };
 1652:     for (index, parameter) in parameters.iter().enumerate() {
 1653:         if kind(parameter) != "ParmVarDecl" || !children(parameter).is_empty() {
 1654:             return Err("unmodeled callback parameter".into());
 1655:         }
 1656:         let id = parameter["id"]
 1657:             .as_str()
 1658:             .ok_or("missing parameter ID")?
 1659:             .to_owned();
 1660:         let ty = c_type(parameter)?;
 1661:         if expected_signature.map_or_else(
 1662:             || !(ty.ends_with('*') || matches!(ty.as_str(), "int" | "unsigned int" | "long")),
 1663:             |(_, _, types)| ty != types[index],
 1664:         ) {
 1665:             return Err("callback parameter type outside selected Unix profile".into());
 1666:         }
 1667:         importer.locals.insert(id.clone());
 1668:         path.locals.insert(
 1669:             id,
 1670:             (
 1671:                 index,
 1672:                 ty.clone(),
 1673:                 Some(value(ty, SourceExpression::Parameter { index })),
 1674:             ),
 1675:         );
 1676:     }
 1677:     // Parse all statements before executing any path; unreachable unsupported
 1678:     // operations still make the entire callback Unknown.
 1679:     let statement = importer.statement(body, 0)?;
 1680:     let paths = execute(&statement, vec![path])?
 1681:         .into_iter()
 1682:         .map(|mut path| {
 1683:             if path.operations.iter().any(|operation| matches!(operation, SourceOperation::ScanLoop { early_return, .. } if early_return.result.c_type != return_type)) {
 1684:                 return Err("scan early return type differs from function signature".into());
 1685:             }
 1686:             if path.terminated {
 1687:                 path.push(SourceOperation::Terminate)?;
 1688:             } else if return_type == "void" {
 1689:                 if path.return_value.is_some() {
 1690:                     return Err("value returned from void function".into());
 1691:                 }
 1692:                 path.push(SourceOperation::Return)?;
 1693:             } else {
 1694:                 let value = path
 1695:                     .return_value
 1696:                     .take()
 1697:                     .ok_or("missing nonvoid return on a source path")?;
 1698:                 check_source_rule("source_type_match", &[("type_matches", i128::from(value.c_type == return_type))])?;
 1699:                 path.push(SourceOperation::ReturnValue { value })?;
 1700:             }
 1701:             let path = SourcePath {
 1702:                 operations: path.operations,
 1703:             };
 1704:             check_path_symbols(&path)?;
 1705:             Ok(path)
 1706:         })
 1707:         .collect::<Result<_, String>>()?;
 1708:     Ok(SourceSummary {
 1709:         function: function.into(),
 1710:         source_revision: revision.into(),
 1711:         signature,
 1712:         paths,
 1713:         profile_assumptions: BTreeMap::new(),
 1714:     })
 1715: }
 1716: 
 1717: #[cfg(test)]
 1718: #[path = "../tests/unit/short_circuit_source.rs"]
 1719: mod short_circuit_tests;
 1720: 
 1721: #[cfg(test)]
 1722: #[path = "../tests/unit/source_symbols.rs"]
 1723: mod symbol_tests;
```

<a id="q16"></a>

## Q16 — `crates/qcore-c-gateway/src/typed_access_path.rs:950–1182`

File SHA-256: `dddd56617d0860536214ed79a2aa413a25e0ed84e6b100dbd3d19012beeaa792`

```rust
  950:         match kind(node) {
  951:             "ParenExpr" | "ImplicitCastExpr" => {
  952:                 if kind(node) == "ImplicitCastExpr"
  953:                     && !matches!(
  954:                         node["castKind"].as_str()?,
  955:                         "LValueToRValue" | "NoOp" | "IntegralCast"
  956:                     )
  957:                 {
  958:                     return None;
  959:                 }
  960:                 self.expression(children(node).first()?)
  961:             }
  962:             "CStyleCastExpr"
  963:                 if self.scalar(node)
  964:                     && children(node)
  965:                         .first()
  966:                         .is_some_and(|inner| self.scalar(inner)) =>
  967:             {
  968:                 self.expression(children(node).first()?)
  969:             }
  970:             "IntegerLiteral" | "FloatingLiteral" | "CharacterLiteral" | "GNUNullExpr" => Some(()),
  971:             "DeclRefExpr" if self.scalar(node) => Some(()),
  972:             "UnaryExprOrTypeTraitExpr"
  973:                 if node["name"].as_str() == Some("sizeof")
  974:                     && children(node).is_empty()
  975:                     && node["argType"]["qualType"]
  976:                         .as_str()
  977:                         .is_some_and(|ty| !ty.contains('[')) =>
  978:             {
  979:                 Some(())
  980:             }
  981:             "UnaryOperator"
  982:                 if matches!(
  983:                     node["opcode"].as_str(),
  984:                     Some("+") | Some("-") | Some("!") | Some("~")
  985:                 ) =>
  986:             {
  987:                 self.expression(children(node).first()?)
  988:             }
  989:             "BinaryOperator"
  990:                 if matches!(
  991:                     node["opcode"].as_str(),
  992:                     Some("+")
  993:                         | Some("-")
  994:                         | Some("*")
  995:                         | Some("/")
  996:                         | Some("%")
  997:                         | Some("<<")
  998:                         | Some(">>")
  999:                         | Some("&")
 1000:                         | Some("|")
 1001:                         | Some("^")
 1002:                         | Some("==")
 1003:                         | Some("!=")
 1004:                         | Some("<")
 1005:                         | Some(">")
 1006:                         | Some("<=")
 1007:                         | Some(">=")
 1008:                 ) =>
 1009:             {
 1010:                 let pair = children(node);
 1011:                 if pair.len() != 2 || !self.scalar(&pair[0]) || !self.scalar(&pair[1]) {
 1012:                     return None;
 1013:                 }
 1014:                 self.expression(&pair[0])?;
 1015:                 self.expression(&pair[1])
 1016:             }
 1017:             "MemberExpr" | "ArraySubscriptExpr" => self.path(node, false, false),
 1018:             "UnaryOperator" if node["opcode"].as_str() == Some("*") => {
 1019:                 self.path(node, false, false)
 1020:             }
 1021:             _ => None,
 1022:         }
 1023:     }
 1024: 
 1025:     pub(crate) fn path(&mut self, node: &Value, store: bool, pointer_read: bool) -> Option<()> {
 1026:         let address = std::mem::take(&mut self.address_target) && !store;
 1027:         if !(address || self.scalar(node) || (pointer_read && !store && pointer_value(node))) {
 1028:             return None;
 1029:         }
 1030:         let mut steps = Vec::new();
 1031:         let mut cursor = node;
 1032:         loop {
 1033:             cursor = strip(cursor)?;
 1034:             match kind(cursor) {
 1035:                 "MemberExpr" => {
 1036:                     let field = self
 1037:                         .fields
 1038:                         .get(cursor["referencedMemberDecl"].as_str()?)?
 1039:                         .as_ref()?;
 1040:                     if field.union_owner || field.bitfield {
 1041:                         return None;
 1042:                     }
 1043:                     let member = cursor["name"].as_str()?.to_owned();
 1044:                     let base = children(cursor).first()?;
 1045:                     let arrow = cursor["isArrow"].as_bool() == Some(true);
 1046:                     // Rule 10n: `(&X)->f` is `X.f`.
 1047:                     let (base, arrow) =
 1048:                         crate::typed_position_shapes::arrow_through_address(base, arrow);
 1049:                     let base_type = qual(base)?;
 1050:                     if base_type.contains("union ")
 1051:                         || base_type.contains("volatile")
 1052:                         || base_type.contains("_Atomic")
 1053:                     {
 1054:                         return None;
 1055:                     }
 1056:                     if arrow && !base_type.trim_end().ends_with('*') {
 1057:                         return None;
 1058:                     }
 1059:                     if !arrow && base_type.trim_end().ends_with('*') {
 1060:                         return None;
 1061:                     }
 1062:                     steps.push((if arrow { "->" } else { "." }, member));
 1063:                     cursor = base;
 1064:                 }
 1065:                 "ArraySubscriptExpr" => {
 1066:                     let pair = children(cursor);
 1067:                     if pair.len() != 2 || !self.scalar(&pair[1]) {
 1068:                         return None;
 1069:                     }
 1070:                     self.expression(&pair[1])?;
 1071:                     steps.push(("[]", String::new()));
 1072:                     cursor = &pair[0];
 1073:                 }
 1074:                 "UnaryOperator" if cursor["opcode"].as_str() == Some("*") => {
 1075:                     steps.push(("*", String::new()));
 1076:                     cursor = children(cursor).first()?;
 1077:                 }
 1078:                 "DeclRefExpr" => break,
 1079:                 _ => return None,
 1080:             }
 1081:         }
 1082:         let root = cursor["referencedDecl"]["name"].as_str()?;
 1083:         if !matches!(
 1084:             cursor["referencedDecl"]["kind"].as_str(),
 1085:             Some("VarDecl") | Some("ParmVarDecl")
 1086:         ) {
 1087:             return None;
 1088:         }
 1089:         if steps.is_empty() {
 1090:             return None;
 1091:         }
 1092:         steps.reverse();
 1093:         let first = steps.first()?;
 1094:         if matches!(first.0, "->" | "*" | "[]") {
 1095:             if !self.plan.scalar.bases.contains(&root.to_owned()) {
 1096:                 self.plan.scalar.bases.push(root.to_owned());
 1097:             }
 1098:             if first.0 == "->" && !self.plan.scalar.record_bases.contains(&root.to_owned()) {
 1099:                 self.plan.scalar.record_bases.push(root.to_owned());
 1100:             }
 1101:         } else if !(self.never_bound)(root) {
 1102:             return None;
 1103:         }
 1104:         let mixed_path = steps.iter().any(|step| step.0 == "[]" && steps.len() != 1)
 1105:             || steps.iter().any(|step| step.0 == "*" && steps.len() != 1)
 1106:             || steps.iter().skip(1).any(|step| step.0 != "->");
 1107:         // The existing tracked field-link model names only direct `->`
 1108:         // places. A path through embedded records or indexed objects has
 1109:         // no tracked place representation, so it is admitted only when the
 1110:         // custody closure proves the root can never bind a resource. Then
 1111:         // every loaded pointer is outside this projection's tracked graph.
 1112:         if mixed_path && !(self.never_bound)(root) {
 1113:             return None;
 1114:         }
 1115:         if steps.len() > 1 || first.0 != "." {
 1116:             let members = steps.iter().map(|step| step.1.clone()).collect::<Vec<_>>();
 1117:             let links = if mixed_path {
 1118:                 Vec::new()
 1119:             } else if members.len() > 1 {
 1120:                 members[..members.len() - 1].to_vec()
 1121:             } else {
 1122:                 Vec::new()
 1123:             };
 1124:             self.plan.chains.push(AccessChain {
 1125:                 root: root.to_owned(),
 1126:                 dereferenced_root: first.0 != ".",
 1127:                 root_never_bound: (self.never_bound)(root),
 1128:                 links,
 1129:                 member: members.last()?.clone(),
 1130:             });
 1131:         }
 1132:         if store && matches!(first.0, "*" | "[]") {
 1133:             self.plan.scalar.writes_through_pointer = true;
 1134:         }
 1135:         self.plan.accesses += 1;
 1136:         Some(())
 1137:     }
 1138: }
 1139: 
 1140: pub(crate) fn strip(mut node: &Value) -> Option<&Value> {
 1141:     while matches!(kind(node), "ParenExpr" | "ImplicitCastExpr") {
 1142:         if kind(node) == "ImplicitCastExpr"
 1143:             && !matches!(
 1144:                 node["castKind"].as_str()?,
 1145:                 "LValueToRValue" | "NoOp" | "ArrayToPointerDecay"
 1146:             )
 1147:         {
 1148:             return None;
 1149:         }
 1150:         node = children(node).first()?;
 1151:     }
 1152:     Some(node)
 1153: }
 1154: 
 1155: pub(crate) fn qual(node: &Value) -> Option<&str> {
 1156:     node["type"]["desugaredQualType"]
 1157:         .as_str()
 1158:         .or_else(|| node["type"]["qualType"].as_str())
 1159: }
 1160: 
 1161: pub(crate) fn is_scalar(node: &Value) -> bool {
 1162:     // A missing/opaque desugared spelling is not evidence of a scalar. In
 1163:     // particular Clang may leave a typedef to an anonymous struct/union as
 1164:     // `a_lock_t`; the former negative filter admitted it as an integer.
 1165:     let Some(text) = qual(node) else { return false };
 1166:     let words = text.split_whitespace().collect::<Vec<_>>();
 1167:     let mut base = false;
 1168:     for word in words {
 1169:         match word {
 1170:             "const" | "restrict" => {}
 1171:             "signed" | "unsigned" | "short" | "long" => base = true,
 1172:             "char" | "int" | "float" | "double" | "_Bool" | "bool" | "__int128" => {
 1173:                 base = true;
 1174:             }
 1175:             _ => return false,
 1176:         }
 1177:     }
 1178:     base
 1179: }
 1180: 
 1181: pub(crate) fn is_pointer(node: &Value) -> bool {
 1182:     qual(node).is_some_and(is_pointer_type)
```

<a id="q17"></a>

## Q17 — `crates/qcore-c-gateway/src/typed_access_positions.rs:1–100`

File SHA-256: `3e34f5c6ba051cdf07d6d91df076483cd966d53907c290ff83da2265fedd0977`

```rust
    1: //! Card **W47-ACCESS-PATH-POSITIONS** (plan P3): the Clang-typed access-path
    2: //! reader of [`crate::typed_access_path`] in three more positions — a
    3: //! controlling-expression operand, a `return` expression and a call actual.
    4: //!
    5: //! The per-hop reader is P2's, unchanged, with all of its refusals (unions,
    6: //! bitfields, casts within a path, volatile/atomic, pointer arithmetic, a
    7: //! tracked mixed embedded/indexed path, pointer-valued stores, calls, and
    8: //! missing or ambiguous AST facts). This module adds only the matching of a
    9: //! sub-expression to its unique Clang node and the value forms each position
   10: //! reads:
   11: //!
   12: //! * **Condition operand** ([`StatementIndex::condition`]): a scalar value,
   13: //!   the truthiness of a pointer-valued path read, or a comparison of
   14: //!   pointer operands (a path read, a plain pointer name, a null constant).
   15: //!   No connective is read here: `&&`, `||` and `?:` stay with the condition
   16: //!   desugaring, which applies this reader per operand on the edges where C
   17: //!   evaluates it. The read binds nothing and refines nothing.
   18: //! * **Assignment operand** ([`StatementIndex::assignment`]): `NAME = PATH`
   19: //!   read exactly as P2 reads the statement (a borrowed target-free alias for
   20: //!   a pointer, a forgotten scalar otherwise).
   21: //! * **Return** ([`StatementIndex::returned`]): one scalar value, or one
   22: //!   pointer-valued path read whose every root the body never binds (the
   23: //!   loaded pointer is then outside the tracked graph) and which the
   24: //!   caller's `pointer_root` admits (constraint 5b's `may_bind` premise). An array member decaying to its address is an interior
   25: //!   address — a derived result (rule 5j) — and refuses.
   26: //! * **Call actual** ([`StatementIndex::argument`]): the actual at `index` of
   27: //!   the unique call of `callee` in the statement, read as a scalar value or,
   28: //!   with every root never bound, as a pointer-valued path read.
   29: //!
   30: //! A node is matched by its internal line and its exact token text and must
   31: //! be unique; implicit-cast wrappers share their operand's range and are not
   32: //! candidates of their own.
   33: 
   34: use serde_json::Value;
   35: 
   36: use crate::typed_access_path::{Reader, StatementIndex, StatementPlan, is_pointer};
   37: use crate::typed_c_ast::{children, kind};
   38: use crate::typed_const_pointer_value::pointer_value;
   39: 
   40: /// What a call actual turned out to be.
   41: #[derive(Clone, Debug)]
   42: pub(crate) struct ArgumentPlan {
   43:     pub(crate) plan: StatementPlan,
   44:     /// The value handed to the callee is a pointer read from memory.
   45:     pub(crate) pointer_valued: bool,
   46: }
   47: 
   48: /// Every node kind that is an expression the matcher may name. Statement
   49: /// nodes, declarations and wrappers sharing their operand's range are not.
   50: fn candidate(node: &Value) -> bool {
   51:     let kind = kind(node);
   52:     !matches!(
   53:         kind,
   54:         "ImplicitCastExpr" | "ConstantExpr" | "FullExpr" | "ExprWithCleanups"
   55:     ) && (kind.ends_with("Expr") || kind.ends_with("Operator") || kind.ends_with("Literal"))
   56: }
   57: 
   58: fn expressions<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
   59:     if candidate(node) {
   60:         out.push(node);
   61:     }
   62:     for child in children(node) {
   63:         expressions(child, out);
   64:     }
   65: }
   66: 
   67: /// Every `CallExpr` under `node` whose callee is the plain name `callee`.
   68: fn calls<'a>(node: &'a Value, callee: &str, out: &mut Vec<&'a Value>) {
   69:     if kind(node) == "CallExpr"
   70:         && let Some(target) = children(node).first()
   71:         && let Some(target) = strip_callee(target)
   72:         && kind(target) == "DeclRefExpr"
   73:         && target["referencedDecl"]["name"].as_str() == Some(callee)
   74:         && target["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
   75:     {
   76:         out.push(node);
   77:     }
   78:     for child in children(node) {
   79:         calls(child, callee, out);
   80:     }
   81: }
   82: 
   83: fn strip_callee(mut node: &Value) -> Option<&Value> {
   84:     while kind(node) == "ImplicitCastExpr" {
   85:         if node["castKind"].as_str()? != "FunctionToPointerDecay" {
   86:             return None;
   87:         }
   88:         node = children(node).first()?;
   89:     }
   90:     Some(node)
   91: }
   92: 
   93: /// A null pointer constant through parentheses and null/bit/no-op casts.
   94: fn null_constant(mut node: &Value) -> bool {
   95:     loop {
   96:         match kind(node) {
   97:             "ParenExpr" => {}
   98:             "ImplicitCastExpr" | "CStyleCastExpr"
   99:                 if matches!(
  100:                     node["castKind"].as_str(),
```

<a id="q18"></a>

## Q18 — `crates/qcore-c-gateway/src/aggregate_store.rs:1–165`

File SHA-256: `74c870d8219be33c98bd999cc6f6a4f5a030e8b62e6e2d7c77935740ff391208`

```rust
    1: //! Typed aggregate field stores (O00 blocker **B08**) for the resource
    2: //! projection.
    3: //!
    4: //! Card C02 admitted direct *scalar* field accesses through bases the
    5: //! projection never binds, and card C02-P/O04-ARMS admitted pointer-field
    6: //! *reads* through such bases. Both deliberately left the **destination** side
    7: //! of the lexical gate in [`crate::legacy_c`] untouched: every statement whose
    8: //! assignment destination contains `->`, `.` or `[` keeps its
    9: //! `indirect memory access is outside the resource projection` refusal.
   10: //!
   11: //! That refusal is the binding constraint on the routes-mask arm of
   12: //! `flb_input_chunk_create` (`docs/reports/remaining-o04-arms/README.md` §3.2):
   13: //! the arm is reached only through `ic->chunk = chunk` and eleven sibling
   14: //! stores, and it branches on `ic->routes_mask == NULL`.
   15: //!
   16: //! This module supplies the missing typed premise for exactly one destination
   17: //! shape — a **single** `->` selection on a local aggregate pointer this
   18: //! function acquired — and for the null test on such a field.
   19: //!
   20: //! ## What an admitted store means
   21: //!
   22: //! An admitted store binds the *place* `BASE->FIELD` in the abstract state, in
   23: //! exactly the way a local variable is bound, and records a **field link**
   24: //! `(owner site, field) -> child sites` in [`crate::legacy_c`]'s abstract
   25: //! state. A field link is deliberately **not** the `CONTAINED` relation a
   26: //! `Stored` contract parameter creates:
   27: //!
   28: //! * escaping the aggregate escapes every live linked child, so publishing an
   29: //!   aggregate does not report the resources reachable from it as leaks;
   30: //! * **releasing** the aggregate does *not* cascade. A declared
   31: //!   `resource_released` release names exactly one identity, and a plain C
   32: //!   field store carries no declared destructor obligation, so a child still
   33: //!   live when its aggregate is released stays live and is reported as the
   34: //!   residual it is. This is premise **P-B08-SHALLOW**, recorded in the card
   35: //!   report and *not* proved here;
   36: //! * overwriting a field replaces its link, so a second store never silently
   37: //!   discharges the first resource.
   38: //!
   39: //! ## Premises
   40: //!
   41: //! **On the base** ([`store_bases`], computed once per function body from the
   42: //! same preprocessed token stream the importer projects):
   43: //!
   44: //! 1. **Readable declarations.** [`FunctionTypes::is_readable`] holds; else
   45: //!    nothing at all is admitted.
   46: //! 2. **Typed local aggregate.** `BASE`'s declared type is exactly
   47: //!    `struct TAG *` ([`DeclaredType::RecordPointer`]) and `BASE` is **not** a
   48: //!    formal parameter, so its identity is one this body acquired.
   49: //! 3. **Single acquired binding.** `BASE` is the destination of exactly one
   50: //!    top-level simple assignment in the body, and that assignment's
   51: //!    right-hand side *calls* something — so the aggregate is one this body
   52: //!    obtained, not one it read out of somebody else's memory. An aggregate
   53: //!    bound by a field read, by a copy of another pointer or by any other
   54: //!    expression is refused, which is what keeps card C02-P's pin
   55: //!    `a_store_through_the_field_between_the_read_and_the_call_is_never_admitted`
   56: //!    refusing with its own existing diagnostic.
   57: //! 4. **No escape of the pointer itself.** `BASE` is never address-taken whole,
   58: //!    never incremented or compound-assigned, never dereferenced (`* BASE`),
   59: //!    never subscripted (`BASE [`) and never selected with `.`.
   60: //! 5. **No aliasing of the destination.** `BASE` is never the entire
   61: //!    right-hand side of an assignment, so no other declared name denotes the
   62: //!    same aggregate; and it never appears as a whole argument at an `Out`,
   63: //!    `Stored` or `Removed` position of a *contracted* call, which could
   64: //!    rebind or attach through it invisibly. An **un**contracted call needs no
   65: //!    premise here: its statement is already refused, which poisons every
   66: //!    pointer on that path, so the flow-sensitive owner check below fails
   67: //!    closed at every later store.
   68: //!
   69: //! **On the field** ([`admitted_store`]):
   70: //!
   71: //! 6. The destination tokens are exactly `BASE - > FIELD` — one selection,
   72: //!    written `->`, with nothing before or after it.
   73: //! 7. `TAG` is a `struct` defined exactly once in the unit whose every member
   74: //!    declaration was read, and `FIELD` is exactly one of three classes: a
   75: //!    scalar member ([`TypedAccessModel::scalar_field`]), a member declared
   76: //!    with exactly one `*` over a specifier list the reader resolved
   77: //!    ([`TypedAccessModel::single_pointer_field`]), or — card
   78: //!    **ALIAS-AGGREGATE** — a member declared exactly `void *`
   79: //!    ([`TypedAccessModel::void_pointer_field`]). Unions, bitfields, arrays,
   80: //!    `volatile`/`_Atomic` members, aggregate sub-objects, function pointers,
   81: //!    `const void *`, `void **`, multi-star members and unresolved typedefs
   82: //!    all refuse.
   83: //!
   84: //! **On the stored value** (decided by [`crate::legacy_c`] against
   85: //! [`classify_value`]): a scalar expression into a scalar field, the null
   86: //! pointer constant into a pointer field, a declared pointer variable of
   87: //! *exactly the field's declared record-pointer type* into that field, a
   88: //! declared record-pointer variable of **any** tag into a `void *` field, or
   89: //! the result of a contracted call.
   90: //!
   91: //! ## The opaque member (card ALIAS-AGGREGATE)
   92: //!
   93: //! A `void *` member is admitted as a **store destination only**. Three facts
   94: //! make the asymmetry the sound one, and each is pinned by a test:
   95: //!
   96: //! * a store replaces exactly one pointer value, so the link `(owner, field) ->
   97: //!   child` remains single-valued — the property every use of a field link
   98: //!   depends on. Nothing in that machinery reads the member's declared type:
   99: //!   escape cascades by identity, overwrite replaces by place, and release
  100: //!   deliberately does not cascade (**P-B08-SHALLOW**, unchanged);
  101: //! * a *read* is refused ([`bound_read_actuals`]), because `void *` names no
  102: //!   pointee and there is therefore nothing to match against the callee's
  103: //!   declared parameter type. Admitting one would hand a callee an identity on
  104: //!   a type premise this reader cannot state;
  105: //! * the stored value must still be a **declared record pointer**, so the
  106: //!   identity written into the member is one this projection can name. A
  107: //!   scalar, a cast, a call result that is not contracted, an address-of and
  108: //!   every opaque local keep the existing refusal.
  109: //!
  110: //! The measured motivation is recorded in `docs/reports/remaining-alias-aggregate/`:
  111: //! on the production corpus `struct flb_input_chunk` declares `void *chunk`,
  112: //! not `struct cio_chunk *chunk` as the reduced O04 fixture prelude does, so
  113: //! card O04-B08's flagship store never reached production at all.
  114: //!
  115: //! **Flow-sensitively**, at the store itself, [`crate::legacy_c`] additionally
  116: //! requires the aggregate to resolve to exactly one **live** identity this
  117: //! body acquired — never a caller-held one — so a store through a released,
  118: //! escaped, null, unknown or caller-owned aggregate refuses. That is the C00
  119: //! premise in its store direction and it is not weakened here.
  120: //!
  121: //! ## What an admitted bound *read* means (card O04-READ-BOUND)
  122: //!
  123: //! [`bound_read_actuals`] is the read half of the same premise set. Card C02-P
  124: //! admits a member read only through a base the projection **never** binds; a
  125: //! read through a base this body *did* bind stayed refused, which was the named
  126: //! blocker `O04-B08-READ-BOUND-BASE`. It is now admitted for exactly one shape —
  127: //! a whole `Borrowed` actual written `BASE - > FIELD` on a [`store_bases`] base
  128: //! whose member is a single-star pointer — and only when [`crate::legacy_c`]'s
  129: //! transfer function additionally finds, *here*, that the aggregate resolves to
  130: //! one live identity this body acquired and that the member carries the exact
  131: //! field link an admitted store recorded. Everything else keeps its refusal.
  132: //!
  133: //! Both premises card C02 records — declaration completeness and C effective
  134: //! types — are inherited unchanged and are not proved here.
  135: 
  136: use std::collections::BTreeSet;
  137: 
  138: use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
  139: 
  140: use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};
  141: 
  142: /// The local aggregate pointers of one function body a store may target.
  143: #[derive(Clone, Debug, Default)]
  144: pub(crate) struct AggregateStoreBases {
  145:     admissible: BTreeSet<String>,
  146:     /// The strict subset of [`Self::admissible`] that satisfies premise **F**
  147:     /// of card O04-B09B10: no callee of this body can reach the aggregate at
  148:     /// all, except through an argument that is exactly one admitted
  149:     /// `BASE - > FIELD` member read. See [`insulated_bases`].
  150:     insulated: BTreeSet<String>,
  151: }
  152: 
  153: impl AggregateStoreBases {
  154:     pub(crate) fn contains(&self, name: &str) -> bool {
  155:         self.admissible.contains(name)
  156:     }
  157: 
  158:     /// Whether premise **F** holds for `name`: every occurrence of it inside
  159:     /// any call's argument list is exactly one admitted member read, so no
  160:     /// callee ever receives the aggregate, its address, or the address of one
  161:     /// of its members. Only then may a member read of `name` occupy a position
  162:     /// that *takes* custody (card O04-B09B10).
  163:     pub(crate) fn is_insulated(&self, name: &str) -> bool {
  164:         self.insulated.contains(name)
  165:     }
```

<a id="q19"></a>

## Q19 — `crates/qcore-c-gateway/src/legacy_condition_desugar.rs:1–180`

File SHA-256: `15c5c3c9577f4264b5fcba520b4fe2417960c1238bdd4462c66d1f23e3df1a7c`

```rust
    1: //! Card **W44-BRANCH-CONDITION-CALLS**: desugaring a controlling expression
    2: //! that carries modeled calls into a small CFG fragment.
    3: //!
    4: //! # What this module decides
    5: //!
    6: //! Cards B02 and B03 isolate **one** unconditionally evaluated contracted call
    7: //! from a controlling expression ([`crate::legacy_condition_call`]). Every
    8: //! other call-bearing condition — a short-circuit `&&`/`||`, a conditional
    9: //! `?:`, two calls, a locally defined callee, `(c = f(x)) != EOF` — kept the
   10: //! position's refusal (`unmodeled call, aggregate or indirect access in …`).
   11: //!
   12: //! This module lowers such a condition to what C evaluates:
   13: //!
   14: //! 1. The expression is read as a Boolean tree over `&&`, `||`, `!` and a
   15: //!    top-level `?:` ([`parse`]). A **leaf** is a maximal operand that holds no
   16: //!    top-level connective.
   17: //! 2. Every call of a leaf becomes its own node, in C evaluation order, holding
   18: //!    exactly the instruction the ordinary statement classifier gives the call
   19: //!    as a statement (`Parser::classify_simple`). There is one call-effect
   20: //!    model; this module adds none.
   21: //! 3. The leaf then decides through one branch node: `NoOp` for a
   22: //!    custody-neutral residual, or the existing null / validity / distinct /
   23: //!    typed-read branch for a call-free leaf.
   24: //! 4. The tree's connectives become **edges** ([`ConditionPlan`]): `A && B`
   25: //!    evaluates `B` only on `A`'s true edge, `A || B` only on `A`'s false edge,
   26: //!    `C ? A : B` evaluates `A` only on `C`'s true edge and `B` only on its
   27: //!    false edge, and `!A` swaps `A`'s two exits. A call C skips is therefore
   28: //!    never applied on a path where C skipped it, and a call C evaluates is
   29: //!    applied on every path where C evaluates it — the evaluation-frequency
   30: //!    theorem of `QCore.ConditionCallDesugar`.
   31: //!
   32: //! # Fresh temporaries
   33: //!
   34: //! Each call's value is conceptually a fresh temporary. An admitted call's
   35: //! result is resource-free: its contract declares `resource_return_none()` or a
   36: //! derived (non-owning) result, and the classifier itself refuses a discarded
   37: //! owned result. A temporary that tracks nothing and is read by no refinement
   38: //! binds nothing in the projection, so the node carries no destination — the
   39: //! same lowering card B02 gives its one call. A custody result (owned,
   40: //! reallocated, a descriptor) is **refused**, so `if ((p = malloc(n)) == NULL)`
   41: //! keeps exactly the refusal it has today.
   42: //!
   43: //! The one written temporary is the C program's own: `(NAME = CALL(…))`, with
   44: //! `NAME` a declared scalar or single-level pointer local that is no formal and
   45: //! no descriptor name, lowers to the statement `NAME = CALL(…)` — the
   46: //! classifier's `ContractCall` with destination `NAME`, binding whatever that
   47: //! statement binds (an untracked value, or a derived pointer tracked by its
   48: //! base) — and the leaf then decides on `NAME` exactly as a following
   49: //! `if (NAME …)` would. `NAME` may not be read elsewhere in the leaf (C leaves
   50: //! that unsequenced). An owned result assigned this way is custody and refuses.
   51: //!
   52: //! # Refusals (fail-closed)
   53: //!
   54: //! Any of the following keeps the whole condition refused, byte for byte:
   55: //! braces, a type operator anywhere, an increment or decrement, the GNU `?:`
   56: //! elvis operator, a call inside a connective nested below a leaf, a call
   57: //! through a member or an expression, an uncontracted or refused call, a custody
   58: //! result, a callee in the enclosing function's own recursive component (whose
   59: //! contract may be a provisional assumption of the neutral route, constraint
   60: //! 4), two calls in one leaf unless every one is the identity on the abstract
   61: //! state (C leaves their order unspecified), a residual that is not
   62: //! custody-neutral, a residual or call-free leaf reading a tracked descriptor
   63: //! outside a validity test, and a switch expression that is not a single leaf.
   64: //!
   65: //! A declined plan restores `next_site` and the element-lowering counter, so a
   66: //! condition this rule keeps refused keeps its site numbering and certificate
   67: //! identity.
   68: 
   69: use std::collections::{BTreeMap, BTreeSet};
   70: 
   71: use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};
   72: 
   73: use super::{
   74:     CfgBuilder, LegacyInstruction, LegacyNodeId, Parser, Statement, SwitchTargets, TYPE_OPERATORS,
   75:     Token, contains_braces, contains_call, descriptor_custody, is_identifier, matching,
   76: };
   77: use crate::neutral_position::ExpressionRole;
   78: 
   79: /// Whether the desugaring is consulted by the lowering. The census commits
   80: /// (`2a4fe098`, `45d4c5ce`, `df5edb61`) shipped the recogniser with this
   81: /// `false`, so the census answers are the rule's own while nothing was
   82: /// admitted; the admission commit sets it.
   83: pub(super) const ADMIT: bool = true;
   84: 
   85: /// The controlling-expression position a plan is built for.
   86: #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   87: pub(super) enum Position {
   88:     Branch,
   89:     Loop,
   90:     ForLoop,
   91:     DoWhile,
   92:     Switch,
   93: }
   94: 
   95: impl Position {
   96:     #[cfg(feature = "indirect-census")]
   97:     fn label(self) -> &'static str {
   98:         match self {
   99:             Self::Branch => "if",
  100:             Self::Loop => "while",
  101:             Self::ForLoop => "for",
  102:             Self::DoWhile => "do-while",
  103:             Self::Switch => "switch",
  104:         }
  105:     }
  106: }
  107: 
  108: /// Where one plan edge leads.
  109: #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  110: pub(super) enum Target {
  111:     /// The condition's true exit; for a switch, the dispatch to every case.
  112:     True,
  113:     /// The condition's false exit (the loop exit in a loop position).
  114:     False,
  115:     /// Another node of the same plan.
  116:     Node(usize),
  117: }
  118: 
  119: /// One node of a desugared condition.
  120: #[derive(Clone, Debug)]
  121: pub(super) struct PlanNode {
  122:     pub(super) instruction: LegacyInstruction,
  123:     pub(super) successors: Vec<Target>,
  124: }
  125: 
  126: /// A desugared controlling expression: calls in evaluation order and one
  127: /// branch node per leaf, wired by the Boolean connectives.
  128: #[derive(Clone, Debug)]
  129: pub(super) struct ConditionPlan {
  130:     pub(super) nodes: Vec<PlanNode>,
  131:     pub(super) entry: usize,
  132: }
  133: 
  134: /// Why a call-bearing condition keeps its refusal. Census vocabulary.
  135: #[derive(Clone, Debug, PartialEq, Eq)]
  136: pub(super) enum Refusal {
  137:     /// The expression carries no call; recorded by the census only.
  138:     #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
  139:     NoCall,
  140:     Braces,
  141:     TypeOperator,
  142:     IncrementOrDecrement,
  143:     Elvis,
  144:     Unparsed,
  145:     SwitchShape,
  146:     NestedConnective,
  147:     IndirectCallee,
  148:     AssignmentShape,
  149:     AssignmentDestination(String),
  150:     RecursiveLocalCallee(String),
  151:     CallRefused(String, String),
  152:     CustodyResult(String),
  153:     UnsequencedEffects,
  154:     ResidualRefused,
  155:     DescriptorRead(String),
  156:     CallFreeLeafRefused,
  157: }
  158: 
  159: impl Refusal {
  160:     #[cfg(feature = "indirect-census")]
  161:     fn code(&self) -> (&'static str, Option<String>, Option<String>) {
  162:         match self {
  163:             Self::NoCall => ("no-call", None, None),
  164:             Self::Braces => ("braces", None, None),
  165:             Self::TypeOperator => ("type-operator", None, None),
  166:             Self::IncrementOrDecrement => ("increment-or-decrement", None, None),
  167:             Self::Elvis => ("elvis", None, None),
  168:             Self::Unparsed => ("unparsed", None, None),
  169:             Self::SwitchShape => ("switch-shape", None, None),
  170:             Self::NestedConnective => ("nested-connective", None, None),
  171:             Self::IndirectCallee => ("indirect-callee", None, None),
  172:             Self::AssignmentShape => ("assignment-shape", None, None),
  173:             Self::AssignmentDestination(name) => {
  174:                 ("assignment-destination", Some(name.clone()), None)
  175:             }
  176:             Self::RecursiveLocalCallee(name) => {
  177:                 ("recursive-local-callee", Some(name.clone()), None)
  178:             }
  179:             Self::CallRefused(name, why) => ("call-refused", Some(name.clone()), Some(why.clone())),
  180:             Self::CustodyResult(name) => ("custody-result", Some(name.clone()), None),
```

<a id="q20"></a>

## Q20 — `crates/qcore-c-gateway/src/variadic_nested.rs:1–105`

File SHA-256: `f2155f950b8f5217d60ea00f27fa0988a455b4e3fe97ee1cc97a66bdc38f11e9`

```rust
    1: //! Declared variadic data tails and one level of nested calls (card
    2: //! **O04-TRACE**).
    3: //!
    4: //! These are the two families card O04-B09B10 measured to be jointly the whole
    5: //! residual of the constructor's success exit **C6**, in one statement:
    6: //!
    7: //! ```c
    8: //! flb_trace("… '%s' … '%s'", flb_input_chunk_get_name(ic), tag);
    9: //! ```
   10: //!
   11: //! `flb_trace` is `void flb_trace(const char *, ...)`, which the contract
   12: //! vocabulary could not spell (card O04-ARMS's **O04-ARMS-VARIADIC**), and its
   13: //! second argument is a **call**, which card O04-B09B10 turned into an explicit
   14: //! refusal (**O04-B09B10-NESTED**). Neither family is a variation of the other,
   15: //! so both land here and must compose.
   16: //!
   17: //! # What each family admits
   18: //!
   19: //! ## The variadic data tail
   20: //!
   21: //! `qcore_contract::contract_loader` accepts
   22: //! `ensures normal resource_variadic_data_tail();` on a declaration. It is a
   23: //! **trusted assumption** that the callee takes no custody of, and retains no
   24: //! pointer to, any argument beyond the ones the declaration names. It says
   25: //! nothing about the callee's body, nothing about format strings, and it
   26: //! **admits nothing on its own**: this module still requires, per call site,
   27: //! that every tail actual really is data
   28: //! ([`tail_arguments_are_data`], premises T1-T4). An actual carrying a tracked
   29: //! resource identity refuses under the declaration exactly as it does without
   30: //! one — the declaration covers what it covers.
   31: //!
   32: //! Card **NEIGHBOURS-CHAINED-TAIL** adds one clause, **T5**: a tail actual that
   33: //! is exactly one *admitted bounded typed access chain* is data. The chain half
   34: //! is [`crate::borrowed_actual::is_admitted_access_chain`] — byte-for-byte the
   35: //! predicate that already decides a declared `Borrowed` actual — so the only
   36: //! thing the tail position contributes is the declaration, and the declaration
   37: //! is read from the callee, never from the actual. See
   38: //! [`is_admitted_tail_access`] for the argument and for what stays refused.
   39: //!
   40: //! Card **W44-LIBC-RESIDUALS** adds **T6**, the one admission that needs
   41: //! neither the enclosing function's typed declarations nor C02-P's closure: a
   42: //! tail actual that is only literals and C11 6.4.2.2 predefined identifiers is
   43: //! data, because no declaration or binding can reach either. See
   44: //! [`is_literal_or_predefined_actual`]; `QCore.LiteralTailActual` is the Lean
   45: //! model.
   46: //!
   47: //! Without a declared tail, an actual beyond the declared parameter vector is
   48: //! now **refused**. That is a fail-closed repair: the modeling zips the
   49: //! contract's parameter vector against the actuals, so before this card an
   50: //! excess actual was silently dropped, exactly as card O04-B09B10 found a
   51: //! nested call's effects being dropped.
   52: //!
   53: //! ## One level of nesting
   54: //!
   55: //! [`admitted_nested_calls`] admits any number of sibling nested calls, each
   56: //! under nine conjunctive premises. The load-bearing pair is **N6** and **N7**:
   57: //! every nested
   58: //! contract's result mode is `None` and every one of its parameter modes is
   59: //! `Borrowed`. Under those two the nested call's **declared effects are the
   60: //! identity on the abstract state**, which is what makes composing it with the
   61: //! outer call well defined at all: C leaves the evaluation order of a call's
   62: //! arguments unspecified (C11 6.5.2.2p10), so an inner call with any custody
   63: //! effect would have to be ordered against the outer call's other arguments,
   64: //! and this projection has no such order. An admitted inner call needs no
   65: //! order because it changes nothing.
   66: //!
   67: //! The result mode is where "data-only or a borrowed handback" resolves: the v1
   68: //! vocabulary has exactly **one** non-custody result mode, `resource_return_none()`,
   69: //! so both phrasings are the same premise. A borrowed handback's aliasing into
   70: //! the callee's argument is deliberately **not** modeled — the result is not
   71: //! tracked at all — which is why premise **N8** requires the result to occupy a
   72: //! position that needs no identity: a declared `Borrowed` parameter, or a
   73: //! declared data tail. It is never bound to a name, so it cannot outlive the
   74: //! statement.
   75: //!
   76: //! # What is refused, by construction
   77: //!
   78: //! Two-level nesting, a nested call whose callee has no contract in force, a nested result carrying custody, a nested
   79: //! parameter carrying custody, a nested call that is only *part* of an
   80: //! argument, a nested call inside the nested call's own arguments, an inner
   81: //! actual this projection can neither denote nor call data (premise **N9**, the
   82: //! fail-open this card found and closed — see `admitted_nested_calls`), a tail
   83: //! actual mentioning a name that may hold a tracked identity, every tail actual
   84: //! carrying `&`, `*` or a subscript, and every tail actual carrying a member
   85: //! access that is not *exactly* one admitted chain (T5).
   86: //!
   87: //! # Inherited premises, not proved here
   88: //!
   89: //! C02's declaration completeness and C effective types; C02-P's
   90: //! `untracked_bases` closure and its recorded may-alias boundary
   91: //! (**C02-P-ALIAS**), which is what decides whether a tail actual can hold an
   92: //! identity; and that a declared `Borrowed` mode really borrows.
   93: 
   94: use qcore_schemas::contract::{
   95:     ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
   96: };
   97: 
   98: use crate::borrowed_actual::UntrackedBases;
   99: use crate::typed_field_access::{FunctionTypes, TypedAccessModel, is_name};
  100: 
  101: /// One admitted nested call: the inner call, the outer argument it *is*, and
  102: /// its own modeled argument places.
  103: #[derive(Clone, Debug, PartialEq, Eq)]
  104: pub(crate) struct NestedCallActual {
  105:     /// The outer call's argument index this nested call occupies whole.
```

<a id="q21"></a>

## Q21 — `crates/qcore-c-gateway/src/legacy_generic_selection.rs:1–91`

File SHA-256: `296cc00e57c57bc0c5227358240add18d2248ba36773895862c4c736b2aa2083`

```rust
    1: //! C11 generic selections (card **W44-C11-GENERIC-SELECTION**).
    2: //!
    3: //! # What C says
    4: //!
    5: //! `_Generic (controlling-expression, T1: e1, …, default: ed)` is a primary
    6: //! expression (C11 §6.5.1.1). Its controlling expression is **not evaluated**:
    7: //! only its type, after lvalue conversion, is read, and it selects exactly one
    8: //! association at translation time. The result is the selected association's
    9: //! expression, and only that expression is evaluated.
   10: //!
   11: //! # Where the corpus meets it
   12: //!
   13: //! glibc 2.43 defines, under `__GLIBC_USE (ISOC23)` (which `_GNU_SOURCE`
   14: //! enables) and C11 or later, a const-preserving macro for `memchr`, `strchr`,
   15: //! `strrchr`, `strpbrk`, `strstr`, `bsearch` and five wide-character searches
   16: //! (`sys/cdefs.h`, `__glibc_const_generic`):
   17: //!
   18: //! ```text
   19: //! _Generic (0 ? (PTR) : (void *) 1, const void *: (CTYPE) (CALL), default: CALL)
   20: //! ```
   21: //!
   22: //! `CALL` is the function call itself (the macro's own name is not re-expanded
   23: //! inside its replacement list). Before this card every such selection reached
   24: //! the classifier as a call to an uncontracted function `_Generic`, or as an
   25: //! unmodeled call in the condition, return or argument it sat in.
   26: //!
   27: //! # The route: selection invariance
   28: //!
   29: //! The analyzer has no C type checker, so it cannot tell which association the
   30: //! compiler selects. It does not need to when every association is **the same
   31: //! expression up to value-preserving pointer casts**: whichever association is
   32: //! selected, the effects evaluated are that expression's, and the value is the
   33: //! same pointer. [`resolve_in_body`] therefore lowers such a selection to that
   34: //! common expression — for glibc's shape, exactly the text a pre-C23 header
   35: //! produced — and drops the controlling expression, which C does not evaluate.
   36: //! The ordinary classifier then reads the call and applies its declared
   37: //! contract (a derived `strchr` result stays a derived result, with no cast to
   38: //! carry it through).
   39: //!
   40: //! A selection is admitted only when all of the following hold, and is left
   41: //! **byte-for-byte as it was** otherwise, so it keeps whatever refusal it had:
   42: //!
   43: //! 1. it is `_Generic ( control , association {, association} )` with a
   44: //!    nonempty controlling expression, every association `key : expression`
   45: //!    with both halves nonempty, and at most one `default`;
   46: //! 2. after removing enclosing parentheses and leading casts, every
   47: //!    association's expression is token-for-token the same core. A cast is
   48: //!    removed only when its type name is a **pointer** type name (identifiers
   49: //!    followed by `*`s and pointer qualifiers, nothing else) and its operand is
   50: //!    a whole parenthesised group, a direct call or a single name, so the cast
   51: //!    applies to the whole rest of the association and not to a prefix of it
   52: //!    (`(T *) a + b` is never read as a cast of `a + b`);
   53: //! 3. if any cast was removed, the core is a direct call `NAME ( … )` and this
   54: //!    unit's own C declaration of `NAME` reads, unambiguously, as returning an
   55: //!    object pointer ([`declared_abi::declares_object_pointer_result`]), so
   56: //!    every removed cast converted a pointer to a pointer.
   57: //!
   58: //! Selections nested in an association are resolved first; a controlling
   59: //! expression is never rewritten, since it is dropped whole. A refused outer
   60: //! selection is emitted unchanged, inner selections included.
   61: //!
   62: //! # Why the lowering is sound
   63: //!
   64: //! Evaluating the selected association evaluates its core exactly once (a cast
   65: //! and a parenthesis evaluate nothing) and yields the core's value converted by
   66: //! the removed casts. Each removed cast converts an object pointer to an object
   67: //! pointer, which leaves the designated object unchanged (C11 §6.3.2.3p2, p7)
   68: //! under the no-undefined-behaviour object model the gateway already assumes
   69: //! (`AOM-POISON-NO-UB`). So every association has the core's effects and the
   70: //! core's value, whichever the compiler selects. `QCore.GenericSelection`
   71: //! proves this of its model: `an_admitted_association_evaluates_as_the_core`,
   72: //! `the_lowered_effect_is_independent_of_the_selection` and, for the rewrite as
   73: //! a whole, `resolve_preserves_outcomes`.
   74: //!
   75: //! The equality in 2 is equality of the analyzer's own token view. That view
   76: //! collapses every string and character literal to one token, so two cores
   77: //! that differ only in a literal compare equal; the analyzer lowers both
   78: //! identically, because it reads nothing else, and its lowering of either
   79: //! covers the concrete one selected. This is the tokenizer premise of trust
   80: //! boundary 28, already stated for `QCore.LiteralTailActual`.
   81: //!
   82: //! # What it does not do
   83: //!
   84: //! It does not reproduce the compiler's selection (card route 2, reading
   85: //! Clang's `GenericSelectionExpr`, was not built: the ABI and mode readers are
   86: //! kept off the typed AST so that W4 derivation, W4 validation and certificate
   87: //! rechecking reproduce every decision from the preprocessed tokens, and route 1
   88: //! already admits every corpus occurrence). It does not widen what the
   89: //! classifier admits: the lowered core meets exactly the rules it would meet if
   90: //! the source had spelled it.
   91: 
```

<a id="q22"></a>

## Q22 — `crates/qcore-c-gateway/src/type_query.rs:1–60`

File SHA-256: `3c2902d29bf3dcc64eb317071598f1522e8e08b0f91f92dc6126f70222363169`

```rust
    1: //! Fixed scalar C type queries for the custody projection.
    2: //!
    3: //! C11 6.5.3.4 makes `sizeof` of a non-VLA operand and `_Alignof`
    4: //! unevaluated. Admit only a readable scalar type name, or (for `sizeof`)
    5: //! one declared scalar local. Arrays, pointer declarators, calls, compound
    6: //! operands and GNU type operators remain outside this rule. In particular,
    7: //! a typedef must resolve through the existing declaration reader: spelling
    8: //! something inside `sizeof` does not by itself make it unevaluated.
    9: //!
   10: //! The replacement `1` is an operand-class marker, NEVER a computed size or
   11: //! alignment. Consumers may only ask custody-neutrality predicates about the
   12: //! resulting tokens, never use them to build a CFG or refine a branch.
   13: 
   14: use crate::typed_field_access::{FunctionTypes, TypedAccessModel};
   15: 
   16: pub(crate) fn scalar_operands<'a>(
   17:     texts: &[&'a str],
   18:     model: &TypedAccessModel,
   19:     types: &FunctionTypes,
   20: ) -> Option<Vec<&'a str>> {
   21:     if !types.is_readable()
   22:         || !texts
   23:             .iter()
   24:             .any(|text| matches!(*text, "sizeof" | "_Alignof"))
   25:     {
   26:         return None;
   27:     }
   28:     let mut result = Vec::new();
   29:     let mut cursor = 0;
   30:     let mut found = false;
   31:     while cursor < texts.len() {
   32:         let operator = texts[cursor];
   33:         if !matches!(operator, "sizeof" | "_Alignof") {
   34:             result.push(operator);
   35:             cursor += 1;
   36:             continue;
   37:         }
   38:         let start = cursor + 1;
   39:         let (operand, end) = if texts.get(start) == Some(&"(") {
   40:             // No admitted operand contains parentheses itself. Stopping at
   41:             // the first ')' is consequently total-or-reject, not truncation.
   42:             let close = start + texts[start..].iter().position(|text| *text == ")")?;
   43:             (&texts[start + 1..close], close + 1)
   44:         } else if operator == "sizeof" {
   45:             (texts.get(start..start + 1)?, start + 1)
   46:         } else {
   47:             return None;
   48:         };
   49:         let type_name = texts.get(start) == Some(&"(") && model.scalar_type_name(operand);
   50:         let scalar_name =
   51:             operator == "sizeof" && matches!(operand, [name] if types.declared_is_scalar(name));
   52:         if !type_name && !scalar_name {
   53:             return None;
   54:         }
   55:         result.push("1");
   56:         cursor = end;
   57:         found = true;
   58:     }
   59:     found.then_some(result)
   60: }
```

<a id="q23"></a>

## Q23 — `crates/qcore-c-gateway/src/sizeof_operand.rs:1–52`

File SHA-256: `99bd2653d3bb6e06cbce98549b4d71de0fbc80423331df0016d97c876752aa32`

```rust
    1: //! Card **W44-COLLECTION-ALLOCATION**: unevaluated `sizeof` dereference
    2: //! operands.
    3: //!
    4: //! `v = malloc(n * sizeof(*v))` is the corpus's allocation spelling, and the
    5: //! lexical access gate reads the `*` after `(` or `sizeof` as a dereference.
    6: //! It is not one. C11 6.5.3.4p2: "If the type of the operand is a variable
    7: //! length array type, the operand is evaluated; otherwise, the operand is not
    8: //! evaluated and the result is an integer constant." So `sizeof(*v)` reads
    9: //! nothing, dereferences nothing and writes nothing, whatever `v` holds —
   10: //! unless the operand's type is variably modified.
   11: //!
   12: //! This module recognises exactly these operands, parenthesised or not:
   13: //!
   14: //! * `* NAME` and `* * NAME` (one or more stars);
   15: //! * `NAME [ LITERAL ]` with a decimal literal index.
   16: //!
   17: //! and only when **every** declaration of `NAME` in this function's parameter
   18: //! list and body, as read by the total-or-reject declarator reader
   19: //! ([`crate::typed_field_access::declarator_facts`]), is a pointer declarator
   20: //! with at least as many `*` as the operand removes, **no array dimension**,
   21: //! and a specifier list that
   22: //! [`crate::typed_field_access::TypedAccessModel::never_variably_modified`]
   23: //! resolves to a type that cannot be variably modified (arithmetic, `void`, a
   24: //! tagged aggregate, or a typedef chain ending in one). The type of such an
   25: //! operand is that specifier type under the remaining stars, which is never a
   26: //! variable length array type, so the operand is never evaluated. A name
   27: //! declared nowhere in the function, declared unreadably, declared once as an
   28: //! array, or declared over an unresolved typedef (a block-scope VLA typedef
   29: //! among them) is refused, and the gate keeps its refusal.
   30: //!
   31: //! What an admitted operand changes is only the **lexical gate's view**: the
   32: //! operand is replaced by the scalar marker `1`, never a computed size
   33: //! (constraint 5a), and only for a statement whose first call is an ordinary
   34: //! callee. Everything after the gate — the call's contract, its borrowed and
   35: //! protected actuals, the nested-call rule, the flow analysis — reads the
   36: //! original tokens, where the operand is a data argument that names no place.
   37: //! A statement this module admits nothing in sees the gate byte for byte as
   38: //! before.
   39: 
   40: use std::collections::BTreeMap;
   41: 
   42: use crate::typed_field_access::{TypedAccessModel, declarator_facts};
   43: 
   44: /// The scalar marker an unevaluated operand is replaced by in the gate's view.
   45: /// A class marker, never a value: it is read by no rule but the lexical gate.
   46: pub(crate) const MARKER: &str = "1";
   47: 
   48: /// Per function: the names whose every declaration is a pointer declarator
   49: /// with no array dimension over a type that cannot be variably modified,
   50: /// mapped to the fewest `*` any of those declarations carries.
   51: #[derive(Clone, Debug, Default)]
   52: pub(crate) struct OperandTypes {
```

<a id="q24"></a>

## Q24 — `crates/qcore-c-gateway/src/index_traversal.rs:1–100`

File SHA-256: `1943237a5d4815c2b80c415727df2de48a05174cd0ecd6fae7bcfcd5c093afe1`

```rust
    1: //! Card **W43-INDEX-TRAVERSAL**: the syntactic half of index and pointer
    2: //! traversal.
    3: //!
    4: //! Code that walks a buffer or a string does so by index (`s[i]`, `i++`) or by
    5: //! pointer increment (`p++`, `p += k`, `*p++ = c`). The typed scalar-access
    6: //! rule of card W43-INDIRECT-ACCESS-GENERAL already reads one access; this
    7: //! module recognises the statements that **move** the pointer or the index,
    8: //! so the flow analysis can follow them. Every function here is a pure
    9: //! recogniser over the analyzer's own token texts and the declaration facts
   10: //! of [`crate::typed_field_access`]; none of them admits anything by itself.
   11: //! What a recognised statement does to the abstract state is decided in
   12: //! [`crate::legacy_c`], where it is checked flow-sensitively.
   13: //!
   14: //! The recognisers are deliberately narrow:
   15: //!
   16: //! * a **pointer step** is `P++`, `++P`, `P--`, `--P`, `P += E`, `P -= E`,
   17: //!   `P = Q + E` or `P = Q - E` (the last two also as a declaration
   18: //!   `T * P = Q ± E`), where `P` is a declared `SCALAR *` or `struct T *`
   19: //!   name, `Q` any declared single-level pointer object and `E` a scalar value
   20: //!   with no memory access;
   21: //! * a **stepped access** is one typed scalar access statement in which
   22: //!   `* P ++`/`* P --` (also `* ( P ++ )`) or an index `[ X ++ ]`/`[ ++ X ]`
   23: //!   (and the decrements) appears, with the stepped name nowhere else in the
   24: //!   statement: the access is admitted over the old value and the step or the
   25: //!   index's forgotten fact follows it, which is the order C sequences them
   26: //!   in (C11 6.5.2.4p2: the value computation precedes the side effect, and
   27: //!   the statement's end is a sequence point).
   28: //!
   29: //! Nothing here decides bounds. An offset is a scalar the rule does not
   30: //! evaluate; the only fact kept about it is whether it is a positive integer
   31: //! literal, which is what lets a stepped pointer be told apart from its base.
   32: 
   33: pub(crate) use crate::legacy_c::StepOffset;
   34: use crate::typed_field_access::{
   35:     FunctionTypes, PointerKind, TypedAccessModel, TypedScalarPlan, is_name,
   36:     is_scalar_value_expression, typed_scalar_statement,
   37: };
   38: 
   39: /// One recognised pointer step: `destination = source ± offset`.
   40: #[derive(Clone, Debug, PartialEq, Eq)]
   41: pub(crate) struct PointerStepPlan {
   42:     pub(crate) destination: String,
   43:     pub(crate) source: String,
   44:     pub(crate) offset: StepOffset,
   45: }
   46: 
   47: /// One side effect a stepped access performs after its access.
   48: #[derive(Clone, Debug, PartialEq, Eq)]
   49: pub(crate) enum TraversalEffect {
   50:     /// `* P ++`: the pointer moves after the access through its old value.
   51:     Step(PointerStepPlan),
   52:     /// `[ X ++ ]`: the declared scalar index's old fact is forgotten.
   53:     Forget(String),
   54: }
   55: 
   56: /// Strip parentheses that enclose the whole run.
   57: fn strip_enclosing<'a, 'b>(mut texts: &'b [&'a str]) -> &'b [&'a str] {
   58:     while texts.len() >= 2 && texts[0] == "(" && closes_at(texts, 0) == Some(texts.len() - 1) {
   59:         texts = &texts[1..texts.len() - 1];
   60:     }
   61:     texts
   62: }
   63: 
   64: fn closes_at(texts: &[&str], open: usize) -> Option<usize> {
   65:     let mut depth = 0usize;
   66:     for (index, text) in texts.iter().enumerate().skip(open) {
   67:         match *text {
   68:             "(" => depth += 1,
   69:             ")" => {
   70:                 depth = depth.checked_sub(1)?;
   71:                 if depth == 0 {
   72:                     return Some(index);
   73:                 }
   74:             }
   75:             _ => {}
   76:         }
   77:     }
   78:     None
   79: }
   80: 
   81: /// A positive decimal integer literal, with an optional integer suffix:
   82: /// `1`, `2u`, `16UL`. `0`, `00`, octal-looking and hexadecimal spellings are
   83: /// not claimed.
   84: fn is_positive_literal(texts: &[&str]) -> bool {
   85:     let [literal] = strip_enclosing(texts) else {
   86:         return false;
   87:     };
   88:     let digits = literal.trim_end_matches(['u', 'U', 'l', 'L']);
   89:     !digits.is_empty()
   90:         && digits.bytes().all(|byte| byte.is_ascii_digit())
   91:         && !digits.starts_with('0')
   92: }
   93: 
   94: /// A name the rule may step in place: `SCALAR *` or `struct T *`.
   95: fn steppable(name: &str, types: &FunctionTypes) -> bool {
   96:     matches!(
   97:         types.pointer_kind(name),
   98:         Some(PointerKind::Scalar | PointerKind::Record)
   99:     )
  100: }
```

<a id="q25"></a>

## Q25 — `crates/qcore-c-gateway/src/indexed_collection.rs:1–115`

File SHA-256: `f7a55a35722b08d235e2da8803c9b263fb1b5f9f2270750d89b1803ac0ef4216`

```rust
    1: //! Card **W43-INDEXED-RESOURCE-COLLECTIONS**: indexed resource collections.
    2: //!
    3: //! A heap array this function acquired, whose elements hold owned resources,
    4: //! is a *collection*. Its elements are not tracked one by one: the collection
    5: //! carries a partition of the index line `[0, ∞)` into half-open segments over
    6: //! **symbolic** bounds (`0`, `i`, `i + 1`, `n`, a literal), each segment with
    7: //! one element state:
    8: //!
    9: //! * [`Slot::Unfilled`] — never written by a modeled store; its content is
   10: //!   unknown (a `malloc`'d array holds indeterminate values);
   11: //! * [`Slot::Null`] — holds a null pointer, which owes nothing;
   12: //! * [`Slot::Live`] — each element holds an owned resource (or, when
   13: //!   `may_be_null`, a null pointer instead); the collection owes each one;
   14: //! * [`Slot::Released`] — each element was released through the collection.
   15: //!
   16: //! Everything beyond the last bound is `Unfilled`. The segments are the
   17: //! loop invariants the brief asks for: a fill loop keeps "`[0, i)` live", a
   18: //! release loop keeps "`[0, j)` released, `[j, n)` live", and the partial
   19: //! cleanup releases exactly the filled prefix `[0, i)`.
   20: //!
   21: //! # Semantics of a bound
   22: //!
   23: //! A bound `Var(x, k)` denotes `max(0, x + k)` for the current value of the
   24: //! scalar `x`, and a literal denotes itself: indices are clamped at zero,
   25: //! which is exactly the set of indices a pointer returned by an allocator can
   26: //! be subscripted with without undefined behaviour (C11 6.5.6p8). A segment
   27: //! `[lo, hi)` with `hi <= lo` is empty.
   28: //!
   29: //! # What is proved, and how
   30: //!
   31: //! Every operation needs a *provable* fact about bounds, decided by
   32: //! [`Scalars`]: a name the projection binds to the null constant is zero (the
   33: //! existing `Clear` binding of `i = 0`), and a counted loop's body edge
   34: //! carries `i < n`. Anything that cannot be proved refuses; nothing is
   35: //! assumed. A write to a scalar a bound mentions makes the collection
   36: //! [`Collection::Unknown`], except the modeled step `i++` of an index, which
   37: //! renames `i + k` to `i + (k - 1)`.
   38: //!
   39: //! No bounds claim is made: the collection says which indices owe a release,
   40: //! never that an index is inside the allocation.
   41: //!
   42: //! The Lean model is `QCore.IndexedCollection`.
   43: 
   44: use std::collections::BTreeSet;
   45: 
   46: /// A symbolic index bound.
   47: #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
   48: pub enum Term {
   49:     /// A literal index.
   50:     Const(u64),
   51:     /// `max(0, name + offset)` for the current value of the scalar `name`.
   52:     Var { name: String, offset: i64 },
   53: }
   54: 
   55: impl Term {
   56:     pub fn var(name: &str) -> Self {
   57:         Self::Var {
   58:             name: name.to_owned(),
   59:             offset: 0,
   60:         }
   61:     }
   62: 
   63:     /// This bound plus one.
   64:     #[must_use]
   65:     pub fn succ(&self) -> Self {
   66:         match self {
   67:             Self::Const(value) => Self::Const(value.saturating_add(1)),
   68:             Self::Var { name, offset } => Self::Var {
   69:                 name: name.clone(),
   70:                 offset: offset.saturating_add(1),
   71:             },
   72:         }
   73:     }
   74: 
   75:     /// The scalar this bound reads, if any.
   76:     pub fn name(&self) -> Option<&str> {
   77:         match self {
   78:             Self::Const(_) => None,
   79:             Self::Var { name, .. } => Some(name),
   80:         }
   81:     }
   82: 
   83:     /// The element-place spelling of this index: `i`, `3`, `i+1`.
   84:     pub fn spelling(&self) -> String {
   85:         match self {
   86:             Self::Const(value) => value.to_string(),
   87:             Self::Var { name, offset: 0 } => name.clone(),
   88:             Self::Var { name, offset } if *offset > 0 => format!("{name}+{offset}"),
   89:             Self::Var { name, offset } => format!("{name}{offset}"),
   90:         }
   91:     }
   92: }
   93: 
   94: /// What the projection knows about scalars at one program point.
   95: pub trait Scalars {
   96:     /// `name` is provably zero here.
   97:     fn is_zero(&self, name: &str) -> bool;
   98:     /// `max(0, name)` is provably zero here (`name` may be negative).
   99:     fn clamped_zero(&self, _name: &str) -> bool {
  100:         false
  101:     }
  102:     /// Every bound `t` with `name < t` provably holding here (a counted loop's
  103:     /// body edge).
  104:     fn upper_bounds(&self, name: &str) -> Vec<Term>;
  105: }
  106: 
  107: /// A context that proves nothing: every comparison is syntactic.
  108: pub struct NoFacts;
  109: 
  110: impl Scalars for NoFacts {
  111:     fn is_zero(&self, _: &str) -> bool {
  112:         false
  113:     }
  114:     fn upper_bounds(&self, _: &str) -> Vec<Term> {
  115:         Vec::new()
```

<a id="q26"></a>

## Q26 — `crates/qcore-c-gateway/src/legacy_summary.rs:1–125`

File SHA-256: `8c017d2499e240cf6ca1c4b37689a07cc45f17c3a619c20c2ccffd44a9ae5109`

```rust
    1: //! Interprocedural parameter/return custody summaries for the unrestricted-C
    2: //! gateway (W4 of `docs/GENERALIZATION_PLAN.md`; closes LC01's open boundary in
    3: //! `docs/REEVALUATION_FOLLOWUP.md`).
    4: //!
    5: //! # What this pass produces
    6: //!
    7: //! For each internal function of a translation unit the pass tries to derive a
    8: //! [`ResourceFunctionContract`] — the *same* record a `.qcore` module declares
    9: //! and a provider proposes — describing what the call does to each argument and
   10: //! what custody the result carries. Derived summaries are published as
   11: //! [`ResourceSummaryRecord`]s labelled [`ContractTrust::InferredValidated`], so
   12: //! a consumer routes them by trust label rather than by origin.
   13: //!
   14: //! # The two-step discipline: infer, then validate
   15: //!
   16: //! Inference alone is worthless here: a summary that merely *looks* consistent
   17: //! with a body would let a wrapper hide the very leak LC01 reproduced. Every
   18: //! candidate is therefore re-checked by running the ordinary intraprocedural
   19: //! analyzer a second time, with the entry state the candidate prescribes
   20: //! ([`EntryCustody`]), and demanding that
   21: //!
   22: //! 1. the body is fully modeled and defect-free (`Safe`, no diagnostics) —
   23: //!    any unsupported construct, unresolved alias, or leak rejects it;
   24: //! 2. at least one exit is reachable;
   25: //! 3. every reachable exit disposes of every formal exactly as the candidate
   26: //!    says (released, consumed, or still live), with no disagreement between
   27: //!    exits;
   28: //! 4. the containment attached to sites the caller holds is exactly what the
   29: //!    contract's `Stored` modes declare, because any other transfer into a
   30: //!    caller-held aggregate is invisible at the call site;
   31: //! 5. the returned value matches the declared result mode on every exit, and a
   32: //!    formal's own site is never returned as if it were fresh;
   33: //! 6. every out slot the contract declares holds a distinct fresh live
   34: //!    resource at every exit (or a null constant, if the mode is nullable),
   35: //!    and no formal without an `Out` mode is written through.
   36: //!
   37: //! A candidate that fails any of these yields no summary at all, and the call
   38: //! stays an uncontracted call — that is, `Unknown`. Precision is never bought
   39: //! with an unvalidated claim.
   40: //!
   41: //! # Lattice and refinement
   42: //!
   43: //! Parameter modes are ordered by how much the caller must guarantee:
   44: //! `Borrowed` (nothing) below `Owned` (a live non-null resource across the
   45: //! call), with `Released` and `Consumed` describing effects rather than
   46: //! requirements. The lattice the search actually walks is the *binding
   47: //! pattern* — which formals are seeded with an entry resource at all — because
   48: //! `Owned`, `Consumed`, and `Released` prescribe the same entry state and
   49: //! differ only in the exit disposition they claim.
   50: //!
   51: //! Iteration starts from the empty binding, and this is a soundness
   52: //! requirement rather than a tuning choice: seeding a formal with a live
   53: //! non-null resource also asserts that it is *not null*, which refutes a null
   54: //! test in the body and deletes the failure path a summary has to describe.
   55: //! A formal is bound only after the analysis reports it as an unresolved
   56: //! resource identity — that is, only once the body has been observed treating
   57: //! it as a resource it owns. The binding set only grows, so the loop settles
   58: //! after at most one round per formal. Once it is stable, each bound formal's
   59: //! mode is *read off* its exit disposition (live gives `Owned`, released gives
   60: //! `Released`, consumed gives `Consumed`), unbound formals stay `Borrowed`, and
   61: //! the resulting contract is checked against a fresh analysis run.
   62: //!
   63: //! # What is deliberately not inferred
   64: //!
   65: //! Same-type immutable entry-prefix output aliases are normalized only after
   66: //! complete use classification. The original Out slot still requires all-exit
   67: //! validation; mutable, escaping and branch-local aliases remain unsupported.
   68: //!
   69: //! The inferred vocabulary includes bounded direct `Removed` wrappers: the
   70: //! child starts contained in the distinct non-null owner formal named by a
   71: //! modeled `Removed` call, and validation requires it detached/live on every
   72: //! non-null exit; nullable formals carry no child custody on null paths.
   73: //! General heap-link extraction and alias-selected removal are not inferred.
   74: //! The ordinary live-entry search
   75: //! still takes precedence, so store-then-remove does not gain a false entry
   76: //! containment requirement.
   77: //!
   78: //! Card W43-RESULT-ALIAS-RETURNED adds a third ordinary route, consulted only
   79: //! after the two above publish nothing: a body whose every exit returns NULL or
   80: //! a pointer derived from one borrowed formal gets a validated
   81: //! `ResourceReturnMode::Derived` summary ([`derived`]). The formal is tracked by
   82: //! a name-only site that carries no custody, so the summary's `Borrowed` claim
   83: //! for it is checked, not assumed.
   84: //!
   85: //! Likewise, a formal handed straight back to the caller, a formal whose
   86: //! disposition differs between exits with no reallocation explaining it, an
   87: //! out slot left unwritten on some path, and an out slot holding a value the
   88: //! callee never resolved to non-null all yield no summary. Every one of these
   89: //! leaves the call uncontracted, hence `Unknown`.
   90: //!
   91: //! # Recursion
   92: //!
   93: //! Ordinary inference runs first and never assumes a recursive summary. A
   94: //! bounded second route partitions one immutable explicit integer formal into
   95: //! Zero and NonZero. Zero validates without any SCC contract; NonZero may call
   96: //! literal-zero contexts already validated at rank zero. Explicit unsigned
   97: //! components also admit exact `control - 1` actuals by mutual strong induction
   98: //! over concrete natural counter values (including unsigned width conversion).
   99: //! Only those proved scalar actuals are abstracted in the resource CFG. Equal contracts
  100: //! for both contexts are rechecked over a disjoint combined CFG before they
  101: //! become caller contracts. Declared SCCs, unsupported control syntax, zero
  102: //! cycles and general recursion remain outside this contextual route. This is
  103: //! well-founded ranked validation, not a coinductive assumed contract or a general
  104: //! termination proof. Unreachable recursive edges may already validate on the
  105: //! ordinary route under its explicit resource entry preconditions.
  106: //!
  107: //! A third route, [`neutral`] (card C03's neutral-frame recursion), applies only
  108: //! after the two above have published nothing for an undeclared recursive
  109: //! component. It leaves the control shape completely free and fixes the *effect
  110: //! content* at the bottom element instead: the only contract it ever seeds for a
  111: //! member of its own component is `N(m)` — every formal `Borrowed`, result
  112: //! `None` — and it descends a two-point per-member lattice to the greatest
  113: //! post-fixed point below all-`Neutral`. Because that assumption states no
  114: //! precondition and asserts only negative, prefix-closed facts, the optimistic
  115: //! start is admissible there and nowhere above it; an effectful assumption
  116: //! validates vacuously under divergence and therefore still requires a rank.
  117: //!
  118: //! # Self-dependent records
  119: //!
  120: //! A validated summary is published as independent corroboration of a body. A
  121: //! record derived while a declared contract for a member of the same recursive
  122: //! component was in force is not that: the recursive call site read the
  123: //! declaration back. Such records are withheld from [`CustodyDerivation::records`]
  124: //! (design unit U7). The declaration still governs every call site — no
  125: //! authority is lost — but a certificate consumer can no longer read a
```

<a id="q27"></a>

## Q27 — `crates/qcore-c-gateway/src/legacy_recursion.rs:1–180`

File SHA-256: `e0b2f5c2fa14c0bb55d13b74320df6ee25f6f3ad3b078b85bdfa86dfc4afa5a0`

```rust
    1: //! Private context/rank extraction. Only proved scalar countdown arguments are
    2: //! abstracted; source certificates and all resource effects remain unchanged.
    3: use super::*;
    4: 
    5: pub(crate) struct ContextFunction {
    6:     pub(crate) parameters: Vec<Option<String>>,
    7:     pub(crate) control_index: usize,
    8:     control: String,
    9:     unsigned: bool,
   10:     allowed_control: BTreeSet<usize>,
   11:     countdowns: BTreeSet<usize>,
   12:     pub(crate) dependencies: BTreeSet<String>,
   13: }
   14: 
   15: // Deliberately small ABI grammar: explicit built-in integer controls and flat
   16: // pointer parameters. Typedefs, attributes, volatile/atomic controls and old-style
   17: // declarations are not evidence for the exhaustive integer partition.
   18: fn builtin_integer(words: &[String]) -> bool {
   19:     matches!(
   20:         words
   21:             .iter()
   22:             .map(String::as_str)
   23:             .collect::<Vec<_>>()
   24:             .as_slice(),
   25:         ["int"]
   26:             | ["signed"]
   27:             | ["unsigned"]
   28:             | ["signed", "int"]
   29:             | ["unsigned", "int"]
   30:             | ["short"]
   31:             | ["short", "int"]
   32:             | ["unsigned", "short"]
   33:             | ["unsigned", "short", "int"]
   34:             | ["long"]
   35:             | ["long", "int"]
   36:             | ["unsigned", "long"]
   37:             | ["unsigned", "long", "int"]
   38:             | ["long", "long"]
   39:             | ["long", "long", "int"]
   40:             | ["unsigned", "long", "long"]
   41:             | ["unsigned", "long", "long", "int"]
   42:             | ["char"]
   43:             | ["signed", "char"]
   44:             | ["unsigned", "char"]
   45:             | ["_Bool"]
   46:     )
   47: }
   48: 
   49: fn parameter_types(tokens: &[Token]) -> Option<Vec<Vec<String>>> {
   50:     let names = parse_parameter_declarators(tokens)?;
   51:     let groups = tokens.split(|token| token.text == ",").collect::<Vec<_>>();
   52:     if names.len() != groups.len() {
   53:         return None;
   54:     }
   55:     groups
   56:         .iter()
   57:         .zip(names)
   58:         .map(|(group, name)| {
   59:             let end = group.len().checked_sub(usize::from(name.is_some()))?;
   60:             let words = group[..end]
   61:                 .iter()
   62:                 .map(|token| token.text.clone())
   63:                 .collect::<Vec<_>>();
   64:             if words.is_empty()
   65:                 || words.iter().any(|word| {
   66:                     !matches!(
   67:                         word.as_str(),
   68:                         "void"
   69:                             | "char"
   70:                             | "int"
   71:                             | "short"
   72:                             | "long"
   73:                             | "signed"
   74:                             | "unsigned"
   75:                             | "_Bool"
   76:                             | "*"
   77:                             | "const"
   78:                     )
   79:                 })
   80:             {
   81:                 return None;
   82:             }
   83:             Some(words)
   84:         })
   85:         .collect()
   86: }
   87: 
   88: pub(crate) fn prepare(
   89:     tokens: &[Token],
   90:     component: &[String],
   91: ) -> Option<BTreeMap<String, ContextFunction>> {
   92:     let mut result = BTreeMap::new();
   93:     for function in component {
   94:         let (body, _) = find_function_body(tokens, function)?;
   95:         let end = matching(tokens, body, "{", "}")?;
   96:         let name_index = (0..body).rev().find(|&i| {
   97:             tokens[i].text == *function && tokens.get(i + 1).is_some_and(|t| t.text == "(")
   98:         })?;
   99:         let close = matching(tokens, name_index + 1, "(", ")")?;
  100:         let types = parameter_types(&tokens[name_index + 2..close])?;
  101:         let parameters = function_parameters(tokens, function)?;
  102:         let controls = types
  103:             .iter()
  104:             .enumerate()
  105:             .filter(|(_, words)| builtin_integer(words))
  106:             .map(|(i, _)| i)
  107:             .collect::<Vec<_>>();
  108:         if controls.len() != 1
  109:             || types
  110:                 .iter()
  111:                 .enumerate()
  112:                 .any(|(i, words)| i != controls[0] && !words.contains(&"*".to_owned()))
  113:         {
  114:             return None;
  115:         }
  116:         let control_index = controls[0];
  117:         let control = parameters[control_index].clone()?;
  118:         // All visible top-level prototypes must agree with the definition. C11
  119:         // checks this too; the reduced token entry point needs its own guard.
  120:         let mut depth = 0usize;
  121:         let mut declaration_start = 0usize;
  122:         let mut signature = None;
  123:         for i in 0..tokens.len() {
  124:             if depth == 0 && tokens[i].text == *function {
  125:                 if tokens.get(i + 1)?.text != "(" {
  126:                     return None;
  127:                 }
  128:                 let last = matching(tokens, i + 1, "(", ")")?;
  129:                 if !matches!(tokens.get(last + 1)?.text.as_str(), ";" | "{")
  130:                     || parameter_types(&tokens[i + 2..last])? != types
  131:                 {
  132:                     return None;
  133:                 }
  134:                 let returns = tokens[declaration_start..i]
  135:                     .iter()
  136:                     .filter(|t| !matches!(t.text.as_str(), "extern" | "static" | "inline"))
  137:                     .map(|t| t.text.clone())
  138:                     .collect::<Vec<_>>();
  139:                 if returns.is_empty()
  140:                     || returns.iter().any(|t| {
  141:                         !matches!(
  142:                             t.as_str(),
  143:                             "void"
  144:                                 | "*"
  145:                                 | "int"
  146:                                 | "char"
  147:                                 | "long"
  148:                                 | "short"
  149:                                 | "unsigned"
  150:                                 | "signed"
  151:                                 | "const"
  152:                         )
  153:                     })
  154:                 {
  155:                     return None;
  156:                 }
  157:                 if signature.as_ref().is_some_and(|s| *s != returns) {
  158:                     return None;
  159:                 }
  160:                 signature = Some(returns);
  161:             }
  162:             match tokens[i].text.as_str() {
  163:                 "{" => depth += 1,
  164:                 "}" => {
  165:                     depth = depth.checked_sub(1)?;
  166:                     if depth == 0 {
  167:                         declaration_start = i + 1;
  168:                     }
  169:                 }
  170:                 ";" if depth == 0 => declaration_start = i + 1,
  171:                 _ => {}
  172:             }
  173:         }
  174:         let mut allowed = BTreeSet::new();
  175:         for i in body + 1..end {
  176:             if matches!(
  177:                 tokens[i].text.as_str(),
  178:                 "while"
  179:                     | "for"
  180:                     | "do"
```

<a id="q28"></a>

## Q28 — `crates/qcore-c-gateway/src/legacy_declared_abi.rs:1–140`

File SHA-256: `d8acefe4fadbf7c1d250798e5a89c34cca0155890097c1214ac5df9058a3fe64`

```rust
    1: //! Declared-contract / C-declaration ABI agreement (card **LOADER-ABI**).
    2: //!
    3: //! # The fail-open this closes
    4: //!
    5: //! A resource-contract v1 declaration writes an opaque C ABI projection for
    6: //! every parameter and for the result:
    7: //!
    8: //! ```text
    9: //! extern fn flb_sds_create_len(str: ptr<const, u8>, len: usize) -> ptr<mut, u8>
   10: //! ```
   11: //!
   12: //! Those spellings are checked by `qcore_contract::contract_loader` against its
   13: //! own closed table — and against **nothing else**. They are not carried on
   14: //! `ResourceFunctionContract`, which holds custody modes only, and until this
   15: //! card nothing anywhere compared them against the C declaration of the same
   16: //! function in the translation unit being analysed. A declaration could
   17: //! therefore say `-> unit` for a function that really returns `void *`, or
   18: //! `(p: ptr<mut, u8>, n: i32)` for a C function declared `(int n, void *p)`,
   19: //! and the analyzer would apply its custody modes positionally to actuals whose
   20: //! declared types cannot carry them. Card NEIGHBOURS measured the fixture half
   21: //! of this (a gateway fixture declared `-> u64` beside a C prelude declaring
   22: //! `double`) and named the gap; this module is the gate.
   23: //!
   24: //! The types are not decoration: the loader's admissibility rules are keyed on
   25: //! them. `resource_released(p)` is admitted only when `p` is a pointer
   26: //! projection, `resource_out(p)` only at `ptr<mut, ptr<mut, u8>>`, and a
   27: //! non-`None` result only on a pointer return type. An unfaithful type is
   28: //! therefore exactly what buys a custody mode the real C type could not carry,
   29: //! and a `-> unit` declaration of an allocating function hides an acquisition
   30: //! the projection would otherwise have to account for. Both directions are
   31: //! reproduced in `crates/qcore-c-gateway/tests/loader_abi.rs`.
   32: //!
   33: //! # Where the comparison happens, and why here
   34: //!
   35: //! At **bind time**, per translation unit — not at load time. The loader has no
   36: //! translation unit: the same contract module is loaded for 185 units whose
   37: //! declarations of a callee may differ, and one unit's disagreement must not
   38: //! reject the whole environment for the others. The comparison also has to be
   39: //! reproducible by W4 summary derivation, W4 validation and
   40: //! `check_legacy_safety_certificate`, which hold the preprocessed token stream
   41: //! and the environment's own retained trusted texts and nothing else. Both
   42: //! sides of the comparison are read from exactly those two: the C declarations
   43: //! from the same token stream the importer is already projecting (card C02's
   44: //! rule), and the declared spellings from the environment's retained module
   45: //! texts (`contract_loader::declared_abi_signatures_of`, the same rederivation
   46: //! `declared_variadic_data_tails` and `literal_argument_premises` use).
   47: //!
   48: //! # What a disagreement does
   49: //!
   50: //! It **withdraws the declaration for this translation unit**, in both
   51: //! directions that matter:
   52: //!
   53: //! * every call site is rewritten to a marked callee, so the contract is not
   54: //!   applied at any of them and the statement refuses with a named key — the
   55: //!   technique card HASH-CLAUSE uses for an unmet literal premise, chosen for
   56: //!   the same reason: the withdrawal then holds uniformly at every consumer of
   57: //!   the token stream rather than at whichever contract lookup remembered it;
   58: //! * the name is removed from the environment the derivation seeds, so a
   59: //!   function *defined* in this unit is no longer checked against, and its
   60: //!   callers no longer trust, an interface its own C declaration contradicts.
   61: //!
   62: //! The decision is taken once per environment, at each entry that owns one:
   63: //! `analyze_legacy_c_translation_unit_selected_partial`,
   64: //! `recursive_component_census` and `analyze_legacy_c_tokens`. It is
   65: //! deliberately **not** repeated inside `build_function_cfg`, whose callers all
   66: //! sit below one of those and are handed the tokens and the environment the
   67: //! withdrawal already applied to; re-deciding per function would re-read every
   68: //! declared contract module once per analysed body.
   69: //!
   70: //! Withdrawal is monotone in the refusing direction: an uncontracted call is
   71: //! `Unknown` and poisons the enclosing function's state, so no verdict can move
   72: //! towards `Safe` because of it. An inferred W4 summary may still be derived for
   73: //! a defined function whose declared contract was withdrawn — that is sound
   74: //! because an inferred summary is published only after re-analysis validates
   75: //! it, and it never rests on the withdrawn declaration.
   76: //!
   77: //! # Fail-closed on anything unreadable
   78: //!
   79: //! A source set the loader cannot re-read **poisons** the whole comparison and
   80: //! withdraws every contracted callee, exactly as an unreadable premise set
   81: //! does. That is deliberately not the empty map: an empty map compares nothing
   82: //! and applies every declaration, which is the fail-open direction.
   83: //!
   84: //! # The one boundary this version keeps, recorded rather than hidden
   85: //!
   86: //! A contracted name this reader finds **no** declaration for in the unit is
   87: //! not compared, and its contract still applies. That is the status quo for
   88: //! that name, and it is where the reader's own coverage — not the declaration's
   89: //! faithfulness — decides. It is named **`LOADER-ABI-UNDECLARED`** and measured
   90: //! per unit rather than assumed away; requiring a declaration would refuse
   91: //! every callee whose prototype this reader cannot see, which is a different
   92: //! and larger gate.
   93: //!
   94: //! # What card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF widened, and what it did not
   95: //!
   96: //! Card CONSTRUCTOR-RESIDUALS (§4.2) measured the whole remaining residual of
   97: //! production `flb_input_chunk_create` down to **one family**: three callees
   98: //! this reader would not commit to a parameter type of, and therefore withdrew
   99: //! at every call site with `abi-unreadable-declaration`. Both shapes were
  100: //! reader boundaries rather than disagreements:
  101: //!
  102: //! * a `struct`/`union`/`enum` **defined by the typedef itself** —
  103: //!   `typedef union { … } pthread_mutex_t;` — which the earlier `read_typedefs`
  104: //!   abandoned at the aggregate's own opening brace and never recorded;
  105: //! * a **function-pointer typedef** —
  106: //!   `typedef int (*msgpack_packer_write)(void *, const char *, size_t);` —
  107: //!   whose declarator the earlier reader did not parse.
  108: //!
  109: //! Both are now read. Neither is a guess:
  110: //!
  111: //! * an aggregate definition is an **object type**, and the pointer projection
  112: //!   of a pointer to one reads no tag and no member — `union tag *` was already
  113: //!   readable, and an untagged definition differs from it in nothing this
  114: //!   vocabulary can spell. A *tagged* definition is recorded as the specifier
  115: //!   pair `["struct", tag]` it is exactly equal to, so it composes with a
  116: //!   separate `typedef struct tag tag;` instead of colliding with it. As a
  117: //!   **value** an aggregate still refuses: this vocabulary has no spelling for
  118: //!   one, exactly as it has none for a `struct tag` value.
  119: //! * a pointer to function gets its own projection,
  120: //!   [`FUNCTION_POINTER_ABI_TYPE`], which is not an entry of `POINTER_ABI_TYPES`
  121: //!   and is not spelled `ptr<…>`. The loader's two custody gates key on the
  122: //!   leading word `ptr`, so **no custody mode and no owned result can be written
  123: //!   on it** — the admission is in the refusing direction for exactly the thing
  124: //!   card LOADER-ABI exists to stop, a declared type buying a mode the C type
  125: //!   cannot carry.
  126: //!
  127: //! Two boundaries stay, narrowed rather than deleted, and are named:
  128: //!
  129: //! * **`ABI-FNPTR-OPAQUE`** — `fnptr` does not distinguish two function
  130: //!   pointers of different signatures, exactly as `ptr<mut, u8>` does not
  131: //!   distinguish two object pointers of different pointee types. The opacity is
  132: //!   the vocabulary's, not this reader's, and nothing can be bought with it
  133: //!   because the spelling carries no custody.
  134: //! * **`ABI-FNPTR-DEPTH1`** — a pointer *to* a function pointer refuses. It is
  135: //!   an object pointer and would project into a custody-bearing spelling; no
  136: //!   declaration in scope needs it, so it stays refused rather than admitted
  137: //!   untested.
  138: //!
  139: //! An *inline* function-pointer parameter — `int (*cb)(void *)` written out at
  140: //! the parameter position rather than behind a typedef — also still refuses, as
```

<a id="q29"></a>

## Q29 — `crates/qcore-schemas/src/contract.rs:1–208`

File SHA-256: `88873acd8cdef360b96d5bf1b43ff698d43fab2604e4f48064a8dfe137966a7d`

```rust
    1: //! Resource-contract data model shared by the legacy-C analyzer, the
    2: //! contract loader, and (future) providers.
    3: //!
    4: //! Moved verbatim from `qcore-compiler`'s `legacy_c` module (W0 of
    5: //! `docs/GENERALIZATION_PLAN.md`). Types only: the analyzer and the
    6: //! source-assumption recheck stay in the compiler crate.
    7: 
    8: use std::collections::BTreeMap;
    9: 
   10: use serde::{Deserialize, Serialize};
   11: 
   12: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   13: #[serde(rename_all = "snake_case")]
   14: pub enum ResourceApiKind {
   15:     Acquire,
   16:     Release,
   17:     Escape,
   18:     /// A trusted resource-neutral call. Internal definitions never establish
   19:     /// this contract automatically; a neutral summary for an internal function
   20:     /// has to be inferred and validated like any other (see [`ContractTrust`]).
   21:     Neutral,
   22: }
   23: 
   24: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   25: #[serde(rename_all = "snake_case")]
   26: pub enum ResourceParameterMode {
   27:     /// The call observes the argument without requiring resource custody.
   28:     Borrowed,
   29:     /// The caller must retain a known live resource across the call.
   30:     Owned { nullable: bool },
   31:     /// Custody moves to the callee, without claiming that the callee destroys it.
   32:     Consumed { nullable: bool },
   33:     /// The trusted call destroys the resource.
   34:     Released { nullable: bool },
   35:     /// The call initializes the pointed-to destination with fresh custody.
   36:     Out { nullable: bool },
   37:     /// The callee stores this resource under the resource passed at
   38:     /// `owner_parameter`. Destruction or transfer of the owner cascades to it.
   39:     Stored {
   40:         owner_parameter: usize,
   41:         nullable: bool,
   42:     },
   43:     /// The callee removes this resource from the aggregate passed at
   44:     /// `owner_parameter`, restoring direct custody to the caller.
   45:     Removed {
   46:         owner_parameter: usize,
   47:         nullable: bool,
   48:     },
   49: }
   50: 
   51: #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   52: #[serde(rename_all = "snake_case")]
   53: pub enum ResourceReturnMode {
   54:     #[default]
   55:     None,
   56:     Owned {
   57:         nullable: bool,
   58:     },
   59:     /// On success the result replaces `parameter`; on failure the old resource
   60:     /// remains live. A nullable result therefore requires path refinement
   61:     /// before the replacement can be treated as complete.
   62:     Reallocated {
   63:         parameter: usize,
   64:         nullable: bool,
   65:     },
   66:     /// Card W43-LIBC-RESULT-ALIAS: the result is **derived from** the pointer
   67:     /// passed at `parameter` and owns nothing.
   68:     ///
   69:     /// It never acquires, discharges or transfers custody of the base.
   70:     /// Releasing the result is not a release of the base, and releasing the
   71:     /// base invalidates the result. `nullable` states whether the call may
   72:     /// return NULL in place of a derived pointer; `const_dropped` records that
   73:     /// the base parameter is declared `ptr<const, u8>` while the result is
   74:     /// `ptr<mut, u8>` (`strchr`'s `const char *` in, `char *` out). The loader
   75:     /// admits this mode only on a `Borrowed` pointer parameter and a pointer
   76:     /// result, both of depth one.
   77:     Derived {
   78:         parameter: usize,
   79:         extent: DerivedExtent,
   80:         nullable: bool,
   81:         const_dropped: bool,
   82:     },
   83: }
   84: 
   85: /// Which part of the base object a [`ResourceReturnMode::Derived`] result may
   86: /// designate.
   87: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   88: #[serde(rename_all = "snake_case")]
   89: pub enum DerivedExtent {
   90:     /// The result is the base pointer's own value (`memcpy` returns `dst`).
   91:     Exact,
   92:     /// The result points somewhere into the base object (`strchr`, `memchr`).
   93:     Interior,
   94: }
   95: 
   96: /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: **which** obligation a custody
   97: /// position creates or touches.
   98: ///
   99: /// Before this card every tracked resource had one kind, so `free(fopen(…))`
  100: /// and `fclose(malloc(…))` would both have discharged. A release now discharges
  101: /// only an obligation of its own kind; a release of another kind is a
  102: /// violation *candidate* and discharges nothing.
  103: ///
  104: /// [`ResourceKind::Heap`] is the default, and it is what every declaration that
  105: /// names no kind means, so every contract written before this card keeps its
  106: /// meaning exactly. It is also the kind of every allocation-family wrapper the
  107: /// corpus already declares: those declarations mix allocators (`free`,
  108: /// `cfl_sds_destroy`, `flb_free`) inside one heap kind, as they did before.
  109: ///
  110: /// [`ResourceKind::Descriptor`] is the one kind carried by an **integer** (a C
  111: /// `int` file descriptor, whose failure value is `-1`, not NULL); every other
  112: /// kind is carried by a pointer. See [`ResourceKind::is_integer`].
  113: #[derive(
  114:     Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
  115: )]
  116: #[serde(rename_all = "snake_case")]
  117: pub enum ResourceKind {
  118:     /// An allocation from the C heap or an allocator wrapper over it.
  119:     #[default]
  120:     Heap,
  121:     /// A POSIX file descriptor (`open`, `socket`, `accept4`; `close`).
  122:     Descriptor,
  123:     /// A `FILE *` stream (`fopen`; `fclose`).
  124:     Stream,
  125:     /// A `DIR *` directory stream (`opendir`; `closedir`).
  126:     Directory,
  127: }
  128: 
  129: impl ResourceKind {
  130:     /// Every kind, in declaration order.
  131:     pub const ALL: [Self; 4] = [Self::Heap, Self::Descriptor, Self::Stream, Self::Directory];
  132: 
  133:     #[must_use]
  134:     pub const fn is_heap(&self) -> bool {
  135:         matches!(self, Self::Heap)
  136:     }
  137: 
  138:     /// Whether this kind's identity is carried by an integer rather than a
  139:     /// pointer. Its failure value is then a **sentinel** (`-1`), not NULL, so a
  140:     /// contract states its validity with `sentinel`/`valid` rather than
  141:     /// `nullable`/`nonnull`, and a null test of it (`if (fd)`) refines nothing.
  142:     #[must_use]
  143:     pub const fn is_integer(self) -> bool {
  144:         matches!(self, Self::Descriptor)
  145:     }
  146: 
  147:     /// The contract-module spelling. `heap` is never written: it is spelled by
  148:     /// the absence of a kind clause.
  149:     #[must_use]
  150:     pub const fn spelling(self) -> &'static str {
  151:         match self {
  152:             Self::Heap => "heap",
  153:             Self::Descriptor => "descriptor",
  154:             Self::Stream => "stream",
  155:             Self::Directory => "directory",
  156:         }
  157:     }
  158: 
  159:     /// The kind a contract-module spelling names, if any.
  160:     #[must_use]
  161:     pub fn from_spelling(spelling: &str) -> Option<Self> {
  162:         Self::ALL
  163:             .into_iter()
  164:             .find(|kind| kind.spelling() == spelling)
  165:     }
  166: }
  167: 
  168: #[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
  169: pub struct ResourceFunctionContract {
  170:     pub parameters: Vec<ResourceParameterMode>,
  171:     pub result: ResourceReturnMode,
  172:     /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the **non-heap** kind of each
  173:     /// custody-bearing parameter, by index. A parameter absent here is heap.
  174:     /// Omitted from the serialized form when empty, so every contract without
  175:     /// a kind serializes byte for byte as it did before the card. Never holds
  176:     /// [`ResourceKind::Heap`]: [`Self::with_parameter_kind`] normalizes it away,
  177:     /// so two contracts that mean the same thing compare equal.
  178:     #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  179:     pub parameter_kinds: BTreeMap<usize, ResourceKind>,
  180:     /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the kind of an owned or
  181:     /// reallocated result. Heap by default, and omitted when heap.
  182:     #[serde(default, skip_serializing_if = "ResourceKind::is_heap")]
  183:     pub result_kind: ResourceKind,
  184:     /// Card W45-FB-RETRY-LIFETIME: what the call does to an intrusive
  185:     /// container's membership of the object a `resource_borrowed` parameter
  186:     /// designates (contract specification §C04.8). Omitted when empty, so every
  187:     /// contract without one serializes byte for byte as before.
  188:     #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  189:     pub container_effects: BTreeMap<usize, ContainerEffect>,
  190:     /// Card W45-FB-RETRY-LIFETIME: the counted-user clauses (contract
  191:     /// specification §C04.7). Omitted when empty.
  192:     #[serde(default, skip_serializing_if = "CountedUserClauses::is_empty")]
  193:     pub counted_users: CountedUserClauses,
  194: }
  195: 
  196: /// Card **W45-FB-RETRY-LIFETIME**: an intrusive-container effect on the object
  197: /// an argument designates (contract specification §C04.8).
  198: ///
  199: /// The parameter keeps its `resource_borrowed` custody mode for every rule that
  200: /// reads modes (the ABI projection, the admission of a member address). The
  201: /// effect is what moves custody: a consumer that does not apply it MUST refuse
  202: /// the call rather than read it as a borrow.
  203: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
  204: #[serde(rename_all = "snake_case", tag = "effect")]
  205: pub enum ContainerEffect {
  206:     /// `resource_published(entry, container)`: the object whose member (or
  207:     /// which) `entry` designates is linked into the container reached through
  208:     /// `container_parameter`, and the container holds it from then on.
```

<a id="q30"></a>

## Q30 — `crates/qcore-schemas/src/contract.rs:388–434`

File SHA-256: `88873acd8cdef360b96d5bf1b43ff698d43fab2604e4f48064a8dfe137966a7d`

```rust
  388:         }
  389:     }
  390: }
  391: 
  392: /// How much trust a [`ResourceFunctionContract`] carries.
  393: ///
  394: /// The three producers of a contract — a loaded `.qcore` module, the C
  395: /// gateway's interprocedural inference (W4 of `docs/GENERALIZATION_PLAN.md`),
  396: /// and a verifier plugin — emit the *same* data with different trust labels.
  397: /// The label is not a verdict: `Declared` is a trusted assumption that no
  398: /// checker discharged, `InferredValidated` records that the gateway re-ran its
  399: /// intraprocedural analyzer against the summary, and `Proposed` is unchecked
  400: /// until the certificate firewall validates it.
  401: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
  402: #[serde(rename_all = "snake_case")]
  403: pub enum ContractTrust {
  404:     /// Loaded from a trusted `.qcore` contract module. An assumption.
  405:     Declared,
  406:     /// Inferred from a function body and validated by re-running the
  407:     /// intraprocedural analyzer against the inferred summary.
  408:     InferredValidated,
  409:     /// Proposed by a verifier plugin. Unchecked until the core validates it;
  410:     /// never sufficient on its own for a `Safe` verdict.
  411:     Proposed,
  412:     /// Card W45-CONTRACT-DEFERRED-REVIEW: a declared contract whose text marks
  413:     /// it, or one of its clauses, as **awaiting human review**
  414:     /// (`resource_review_pending`, contract spec §C06).
  415:     ///
  416:     /// It is loaded and validated total-or-reject exactly like a declared
  417:     /// contract, but it is neither an assumption nor evidence: it justifies
  418:     /// neither `Safe` nor `Unsafe`, and no validation can promote it. It
  419:     /// differs from [`Self::Proposed`] on both counts — a proposal awaits the
  420:     /// *core's* validation and is only barred from justifying `Safe`. A person
  421:     /// resolves it by editing the text, which changes its digest.
  422:     PendingHumanReview,
  423: }
  424: 
  425: /// Which part of a declaration a review marker defers (card
  426: /// W45-CONTRACT-DEFERRED-REVIEW).
  427: #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
  428: #[serde(rename_all = "snake_case", tag = "scope")]
  429: pub enum ReviewScope {
  430:     /// `requires resource_review_pending("REASON");` — the whole contract.
  431:     Contract,
  432:     /// `ensures normal resource_review_pending(CLAUSE, "REASON");` — one
  433:     /// clause, recorded as its canonical text (`ensures normal CLAUSE`).
  434:     Clause { clause: String },
```

<a id="q31"></a>

## Q31 — `crates/qcore-contract/src/contract_loader.rs:1–85`

File SHA-256: `1e3e2c3c393a67d4ace7b788ea074b5d3b167c8211461fad607da575a91a44b9`

```rust
    1: //! Total-or-reject bridge from native contract modules to legacy-C custody.
    2: //!
    3: //! # Interpreter selection
    4: //!
    5: //! `the canonical contract corpus` is shared by three consumers with disjoint vocabularies.
    6: //! The **`requires resource_contract_v1` marker is the selector**: a declaration
    7: //! that carries it must map completely into [`ResourceFunctionContract`], and a
    8: //! declaration that does not carry it is rejected here rather than skipped, so
    9: //! asking this loader for a `ResourceContracts` environment over an
   10: //! observation/lifecycle module fails closed instead of silently producing a
   11: //! weaker table. `compiler/tests/contract_loader.rs::
   12: //! every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped` pins
   13: //! that split over the whole corpus.
   14: //!
   15: //! | Clause family | Owner |
   16: //! | --- | --- |
   17: //! | `requires resource_contract_v1` + the `resource_*` predicates below | this loader |
   18: //! | integer/Boolean ghost predicates (`result == 0`, `field_NAME`, `busy`, `lock`, `allocations`, `releases`, `old(...)` arithmetic) | `contract_observation` and the ChunkIO/source interpreters |
   19: //! | named atomic lifecycle propositions (`chunk_is_live`, `arena_is_discharged`, `descriptor_custody_is_discharged_once`, ...) | the mechanism/lifecycle interpreters |
   20: //! | `own<T>` / `borrow<'a, _, T>` parameters, `use` annotations, `effects { .. }` rows, `quant` clauses, `ensures any`, `ensures error(E)`, `result<T, E>` returns, `resource type` / `error` / `axis` declarations | the QCore language front end and checker |
   21: //!
   22: //! These families are not approximated into custody modes. A resource
   23: //! projection of a lifecycle module is written as its own v1 module (as
   24: //! `fluent_bit/resource_memory.qv` projects `fluent_bit/memory.qv`);
   25: //! it is never inferred by dropping predicates this loader cannot check.
   26: //!
   27: //! # Version 1 vocabulary
   28: //!
   29: //! Every parameter and the result must occur exactly once, and all clauses
   30: //! other than the marker must be `ensures normal` calls:
   31: //!
   32: //! - `resource_borrowed(p)` -> Borrowed (no custody requirement).
   33: //! - `resource_{owned,consumed,released,out}(p, nullable|nonnull)` -> that mode.
   34: //! - `resource_{stored,removed}(p, owner, nullable|nonnull)` -> indexed custody.
   35: //! - `resource_return_none()` -> None; `resource_return_owned(nullability)` -> Owned.
   36: //! - `resource_return_reallocated(p, nullability)` -> Reallocated, requiring
   37: //!   an Owned parameter; null failure preserves it, success replaces it once.
   38: //! - `resource_return_alias(p, nullability)` and
   39: //!   `resource_return_interior(p, nullability)` -> Derived (card
   40: //!   W43-LIBC-RESULT-ALIAS): the result is `p`'s own value, or a pointer into
   41: //!   the object `p` points to, and owns nothing. See below.
   42: //! - `resource_variadic_data_tail()` -> the declaration additionally covers
   43: //!   calls that pass *more* actuals than it names. See below.
   44: //! - `resource_kind(p, KIND)` and `resource_return_kind(KIND)` -> the non-heap
   45: //!   kind of a custody position (card W43-DESCRIPTOR-RESOURCE-TRACKING). See
   46: //!   below.
   47: //!
   48: //! and one further **precondition** beside the marker:
   49: //!
   50: //! - `requires resource_literal_argument(p, LITERAL);` -> the declaration
   51: //!   applies only at call sites whose actual at `p` is exactly `LITERAL`. See
   52: //!   below.
   53: //!
   54: //! # The variadic data tail (card O04-TRACE)
   55: //!
   56: //! A C variadic function (`void flb_trace(const char *, ...)`) has no
   57: //! per-argument declaration for its tail, so before this clause the vocabulary
   58: //! could only describe one arity at a time — card O04-ARMS declared `flb_error`
   59: //! at its single call site's arity and recorded the gap as
   60: //! **`O04-ARMS-VARIADIC`**.
   61: //!
   62: //! `ensures normal resource_variadic_data_tail();` is that missing form. It
   63: //! declares, for **every** argument beyond the named parameter vector:
   64: //!
   65: //! * the callee takes **no custody** of it, and
   66: //! * the callee **retains no pointer** to it past the call.
   67: //!
   68: //! **What it does not say.** It is a trusted assumption about the callee's
   69: //! contract, exactly like every other clause here; it verifies nothing about
   70: //! the callee's body, and it is not a claim that the callee is a `printf`-style
   71: //! formatter, that the tail is type-correct against a format string, or that
   72: //! the tail arguments are read at all. It also does not admit anything on its
   73: //! own: a consumer that honours the tail must still establish, per call site,
   74: //! that each tail actual really is data. The C gateway does exactly that
   75: //! (`qcore_c_gateway::variadic_nested`), and an actual that carries a tracked
   76: //! resource identity is refused there even under this declaration — the
   77: //! declaration covers what it covers.
   78: //!
   79: //! The tail flag is **not** carried on [`ResourceFunctionContract`]. It is
   80: //! rederived from the retained trusted source texts by
   81: //! [`variadic_data_tails`], so a contract environment that has no declared
   82: //! sources — an inferred W4 summary, a provider proposal, a directly
   83: //! constructed table — has no tails at all, which is the fail-closed direction.
   84: //!
   85: //! # Derived results (card W43-LIBC-RESULT-ALIAS)
```

<a id="q32"></a>

## Q32 — `crates/qcore-core/src/scalar_calls.rs:1–100`

File SHA-256: `e55733e1202925a669c836893cb14cb8a193f39cef47f602c26c9d52826999ab`

```rust
    1: //! Scalar calls with explicit std.io printing: total type checking, acyclic calls, bounded closed execution.
    2: //! Checked arithmetic has no wraparound or C undefined-behavior interpretation.
    3: use std::collections::BTreeSet;
    4: 
    5: pub const MAX_FUNCTIONS: usize = 256;
    6: pub const MAX_DEPTH: usize = 64;
    7: pub const MAX_NODES: usize = 16384;
    8: pub const MAX_STEPS: usize = 100000;
    9: #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   10: pub enum Type {
   11:     I32,
   12:     Unit,
   13: }
   14: #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   15: pub enum Op {
   16:     Add,
   17:     Sub,
   18:     Mul,
   19:     Div,
   20:     Rem,
   21: }
   22: #[derive(Clone, Debug, PartialEq, Eq)]
   23: pub enum Expr {
   24:     Literal(i32),
   25:     Local(usize),
   26:     Neg(Box<Expr>),
   27:     Binary(Op, Box<Expr>, Box<Expr>),
   28:     Call(usize, Vec<Expr>),
   29: }
   30: #[derive(Clone, Debug, PartialEq, Eq)]
   31: pub enum Statement {
   32:     Let(Expr),
   33:     Evaluate(Expr),
   34:     Print(Expr),
   35: }
   36: #[derive(Clone, Debug, PartialEq, Eq)]
   37: pub struct Function {
   38:     /// None infers the effect; Some(false) is an explicit empty effect row.
   39:     pub declared_io: Option<bool>,
   40:     pub parameters: usize,
   41:     pub result: Type,
   42:     pub statements: Vec<Statement>,
   43:     pub returned: Option<Expr>,
   44: }
   45: #[derive(Clone, Debug, PartialEq, Eq)]
   46: pub struct Program {
   47:     pub functions: Vec<Function>,
   48: }
   49: #[derive(Clone, Debug, PartialEq, Eq)]
   50: pub struct Refusal {
   51:     pub function: usize,
   52:     pub message: String,
   53: }
   54: fn bad(function: usize, message: &str) -> Refusal {
   55:     Refusal {
   56:         function,
   57:         message: message.into(),
   58:     }
   59: }
   60: /// Private checked data prevents callers from altering a checked signature/body.
   61: #[derive(Clone, Debug, PartialEq, Eq)]
   62: pub struct Checked {
   63:     program: Program,
   64:     io_functions: BTreeSet<usize>,
   65: }
   66: #[derive(Clone, Debug, PartialEq, Eq)]
   67: pub struct Observation {
   68:     pub value: Option<i32>,
   69:     pub stdout: Vec<i32>,
   70: }
   71: impl Checked {
   72:     pub fn program(&self) -> &Program {
   73:         &self.program
   74:     }
   75:     /// One closed invocation, not a theorem of overflow freedom for all inputs.
   76:     pub fn run(&self, entry: usize, arguments: &[i32]) -> Result<Option<i32>, Refusal> {
   77:         self.observe(entry, arguments).map(|o| o.value)
   78:     }
   79:     pub fn io_functions(&self) -> &BTreeSet<usize> {
   80:         &self.io_functions
   81:     }
   82:     pub fn observe(&self, entry: usize, arguments: &[i32]) -> Result<Observation, Refusal> {
   83:         let mut budget = MAX_STEPS;
   84:         let mut stdout = Vec::new();
   85:         let value = self.call(entry, arguments, &mut budget, &mut stdout, 0)?;
   86:         Ok(Observation { value, stdout })
   87:     }
   88:     fn call(
   89:         &self,
   90:         id: usize,
   91:         args: &[i32],
   92:         budget: &mut usize,
   93:         stdout: &mut Vec<i32>,
   94:         depth: usize,
   95:     ) -> Result<Option<i32>, Refusal> {
   96:         if depth > MAX_DEPTH {
   97:             return Err(bad(id, "scalar call depth exceeded"));
   98:         }
   99:         let f = self
  100:             .program
```

<a id="q33"></a>

## Q33 — `crates/qcore-core/src/disjoint_footprint.rs:1–153`

File SHA-256: `398aed9ba97fa1ddd24ccb057c9ba5bad1b802ed514b480a78ea006e2a87353d`

```rust
    1: //! `DisjointFootprintsV1` — derived access footprints and alias identity.
    2: //!
    3: //! [`crate::concurrency`] and [`crate::mutex_invariant`] both take an
    4: //! [`AccessFootprint`] on trust: an interval a caller *declares*, recorded on
    5: //! the certificate as [`ConcurrencyAssumption::DeclaredFootprintShape`]. This
    6: //! module is the bounded prototype of the profile that *derives* those
    7: //! intervals instead, so that a later increment can discharge that premise for
    8: //! the shapes it covers.
    9: //!
   10: //! What is decided here:
   11: //!
   12: //! * **Interval bounds and disjointness against one allocation generation.**
   13: //!   A slice names an allocation, the generation it was derived under, and an
   14: //!   element range; the checker computes the byte interval itself, refuses a
   15: //!   range that leaves the declared extent, and decides overlap by interval
   16: //!   arithmetic. A caller may supply the byte interval too, in which case it is
   17: //!   compared, never believed.
   18: //! * **Stale aliases.** Reallocation advances the generation and destruction
   19: //!   ends it; every handle of a retired generation is retired with it and every
   20: //!   later use of one is refused.
   21: //! * **Record fields and bit-fields.** A field footprint exists only under an
   22: //!   explicit, internally validated layout contract. Two fields sharing one
   23: //!   bit-field storage unit are one C11 memory location and are **never**
   24: //!   disjoint, even when their bit ranges are.
   25: //! * **Descriptor aliases.** Two descriptors that name one protocol cell are
   26: //!   never disjoint whatever their byte footprints are, and a memory footprint
   27: //!   and a descriptor footprint are *incomparable*: the checker refuses the
   28: //!   question rather than answering "disjoint".
   29: //!
   30: //! What is deliberately **not** decided, and refused rather than approximated:
   31: //!
   32: //! * disjointness across two allocation identities — that needs an allocator
   33: //!   model this profile does not have, so it is a refusal
   34: //!   ([`FootprintDiagnostic::CrossAllocationDisjointnessUnsupported`]), not a
   35: //!   free "distinct objects do not alias";
   36: //! * unions, flexible array members, variable-length arrays, computed indices,
   37: //!   pointer arithmetic outside a declared extent, nested anonymous aggregates,
   38: //!   type-punned aliases, overlapping copies and atomic reclamation.
   39: //!
   40: //! What stays a premise is named on the certificate: the declared element size
   41: //! and count are the real object extent
   42: //! ([`FootprintAssumption::AllocationExtentDeclared`]), a layout contract is the
   43: //! target ABI's real layout ([`FootprintAssumption::DeclaredRecordLayout`]), and
   44: //! a declared protocol cell is really the shared logical state
   45: //! ([`FootprintAssumption::DeclaredProtocolCellIdentity`]) — stamped for a
   46: //! [`FootprintEvent::DeclareDescriptorAlias`] and **not** for a
   47: //! [`FootprintEvent::DeriveDescriptorAlias`], whose cell a deciding model
   48: //! derived (card `CT42-CELL-ROUTE`).
   49: //! [`FootprintAssumption::DeclaredDisjointness`] is the premise this profile
   50: //! discharges: the checker never emits it and
   51: //! [`recheck_disjoint_footprint_certificate`] refuses any certificate carrying
   52: //! it.
   53: //!
   54: //! This is a model-level module. No production audit route calls it and no
   55: //! other checker composes with it; the one source route that reaches it is
   56: //! `source.socket.v1`'s `disjoint NAME NAME;` question, through
   57: //! [`crate::socket_footprint`] (card `CT42-CELL-ROUTE`), which hands this
   58: //! checker a derived-alias projection and reports its `SharedProtocolCell`
   59: //! (`Q664`) unedited. Composing it with `CheckedMutexInvariantV1` is the
   60: //! follow-on unit recorded in `docs/design/DISJOINT_FOOTPRINT_PROFILE.md`.
   61: 
   62: use std::collections::{BTreeMap, BTreeSet};
   63: use std::fmt;
   64: 
   65: pub use crate::concurrency::AccessFootprint;
   66: 
   67: /// One allocation lifetime. Generation-indexed like a `SharedResourceId`:
   68: /// never a reusable address.
   69: pub type AllocationId = u32;
   70: /// Which generation of an allocation a handle was derived under. A handle of a
   71: /// retired generation is stale forever.
   72: pub type Generation = u32;
   73: /// One derived footprint. Minted once and never reused, retired handles
   74: /// included.
   75: pub type HandleId = u32;
   76: /// A record type named by a layout contract.
   77: pub type RecordId = u32;
   78: /// One member of a record.
   79: pub type FieldId = u32;
   80: /// One OS/library descriptor: a handle, not a byte range.
   81: pub type DescriptorId = u32;
   82: /// The shared logical state two descriptor aliases can both advance.
   83: pub type ProtocolCellId = u32;
   84: 
   85: /// The element shape of an allocation. `Record` requires a layout contract to
   86: /// exist before the allocation is admitted, because its element size is the
   87: /// contract's size.
   88: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
   89: pub enum ElementShape {
   90:     /// A scalar or opaque byte block of this many bytes.
   91:     Scalar { size: u32 },
   92:     /// A record whose byte layout comes from a validated layout contract.
   93:     Record { record: RecordId },
   94: }
   95: 
   96: /// How one field occupies storage. The distinction is C11's memory-location
   97: /// rule, not a naming convention: adjacent bit-fields of one unit are one
   98: /// memory location and cannot be written independently.
   99: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
  100: pub enum FieldStorage {
  101:     /// The field owns its byte range outright.
  102:     Addressable,
  103:     /// The field occupies `bit_width` bits at `bit_offset` inside the storage
  104:     /// unit `unit`. Every field of one unit must declare the same byte range.
  105:     BitfieldUnit {
  106:         unit: u32,
  107:         bit_offset: u32,
  108:         bit_width: u32,
  109:     },
  110: }
  111: 
  112: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
  113: pub struct FieldLayout {
  114:     pub field: FieldId,
  115:     pub byte_offset: u32,
  116:     pub byte_size: u32,
  117:     pub storage: FieldStorage,
  118: }
  119: 
  120: /// An explicit, whole-record layout declaration. There is no partial
  121: /// admission: a contract that fails any structural rule is refused entirely and
  122: /// every field derivation naming that record is refused with it.
  123: #[derive(Clone, Debug, PartialEq, Eq)]
  124: pub struct LayoutContract {
  125:     pub record: RecordId,
  126:     pub size: u32,
  127:     pub fields: Vec<FieldLayout>,
  128: }
  129: 
  130: /// Alias and layout shapes this profile refuses rather than approximates. Each
  131: /// is a real event, so "the profile refuses it" is testable instead of the
  132: /// model quietly lacking a way to say it.
  133: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
  134: pub enum UnsupportedFootprint {
  135:     UnionMember,
  136:     FlexibleArrayMember,
  137:     VariableLengthArray,
  138:     ComputedIndex,
  139:     PointerArithmeticOutsideDeclaredExtent,
  140:     NestedAnonymousAggregate,
  141:     TypePunnedAlias,
  142:     OverlappingCopy,
  143:     CrossAllocationAliasProof,
  144:     AtomicReclamation,
  145: }
  146: 
  147: #[derive(Clone, Debug, PartialEq, Eq)]
  148: pub enum FootprintEvent {
  149:     /// Introduce generation `generation` of `alloc` with `count` elements.
  150:     Allocate {
  151:         alloc: AllocationId,
  152:         generation: Generation,
  153:         shape: ElementShape,
```

<a id="q34"></a>

## Q34 — `crates/qcore-core/src/weak_memory.rs:1–90`

File SHA-256: `4b535cbd1ef3f4cda2760ce1227ab03bf1eb59aff0975646c75205ce4656d61e`

```rust
    1: //! Cycle 17 execution checker for the C11 concurrency boundary.
    2: //!
    3: //! Candidate executions explicitly supply reads-from, modification order,
    4: //! synchronizes-with, and the sequentially-consistent order.  Program order is
    5: //! derived per thread and is never inferred from inter-thread list position.
    6: //! The atomic subset follows C11 N1570 (not C++20/RC11): plain loads/stores
    7: //! and same-thread store release sequences. RMWs, fences, and consume are not
    8: //! represented. Acceptance checks one supplied graph, not program exploration.
    9: 
   10: use std::collections::{BTreeMap, BTreeSet, VecDeque};
   11: use std::fmt;
   12: 
   13: pub type MemoryEventId = u32;
   14: pub type MemoryThreadId = u32;
   15: pub type MemoryLocation = u32;
   16: 
   17: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
   18: pub enum MemoryOrder {
   19:     Relaxed,
   20:     Acquire,
   21:     Release,
   22:     AcquireRelease,
   23:     SequentiallyConsistent,
   24: }
   25: 
   26: impl MemoryOrder {
   27:     const fn has_acquire(self) -> bool {
   28:         matches!(
   29:             self,
   30:             Self::Acquire | Self::AcquireRelease | Self::SequentiallyConsistent
   31:         )
   32:     }
   33: 
   34:     const fn has_release(self) -> bool {
   35:         matches!(
   36:             self,
   37:             Self::Release | Self::AcquireRelease | Self::SequentiallyConsistent
   38:         )
   39:     }
   40: }
   41: 
   42: #[derive(Clone, Debug, PartialEq, Eq)]
   43: pub enum MemoryEventKind {
   44:     AtomicInit {
   45:         location: MemoryLocation,
   46:         value: i64,
   47:     },
   48:     AtomicStore {
   49:         location: MemoryLocation,
   50:         value: i64,
   51:         order: MemoryOrder,
   52:     },
   53:     AtomicLoad {
   54:         location: MemoryLocation,
   55:         value: i64,
   56:         order: MemoryOrder,
   57:     },
   58:     NonAtomicRead {
   59:         location: MemoryLocation,
   60:     },
   61:     NonAtomicWrite {
   62:         location: MemoryLocation,
   63:     },
   64:     MutexUnlock {
   65:         mutex: u32,
   66:     },
   67:     MutexLock {
   68:         mutex: u32,
   69:     },
   70:     Spawn {
   71:         child: MemoryThreadId,
   72:     },
   73:     Join {
   74:         child: MemoryThreadId,
   75:     },
   76: }
   77: 
   78: #[derive(Clone, Debug, PartialEq, Eq)]
   79: pub struct MemoryEvent {
   80:     pub id: MemoryEventId,
   81:     pub thread: MemoryThreadId,
   82:     pub kind: MemoryEventKind,
   83: }
   84: 
   85: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
   86: pub struct RelationEdge {
   87:     pub from: MemoryEventId,
   88:     pub to: MemoryEventId,
   89: }
   90: 
```

<a id="q35"></a>

## Q35 — `crates/qcore-c-gateway/src/atomic_publication_source.rs:1–100`

File SHA-256: `2debce66a585bb99883b022f1c2256322a07b42fffda56e8887643594fba15c1`

```rust
    1: //! Closed, bounded source route for C11 release/acquire publication.
    2: //!
    3: //! Acceptance is conditional: if the consumer takes the branch guarded by the
    4: //! observed ready value, the payload write happens before its payload read.
    5: //! Termination and eventual observation are deliberately outside this route.
    6: 
    7: use serde_json::Value;
    8: use std::collections::BTreeMap;
    9: 
   10: use crate::legacy_c::{C11TranslationUnit, LegacyFunctionScope, validate_unrestricted_c11};
   11: use crate::typed_c_ast::{children, kind, translation_unit_ast};
   12: 
   13: #[derive(Clone, Debug, PartialEq, Eq)]
   14: pub struct AtomicPublicationSourceProfile<'a> {
   15:     pub publisher: &'a str,
   16:     pub consumer: &'a str,
   17:     pub flag: &'a str,
   18:     pub payload: &'a str,
   19: }
   20: 
   21: #[derive(Clone, Debug, PartialEq, Eq)]
   22: pub struct AtomicPublicationExecutionBounds {
   23:     /// Declared premise; a definition cannot prove its dynamic call count.
   24:     pub publisher_invocations: usize,
   25:     pub successful_consumer_invocations: usize,
   26: }
   27: 
   28: #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   29: pub enum AtomicPublicationOrder {
   30:     Relaxed,
   31:     Acquire,
   32:     Release,
   33:     SequentiallyConsistent,
   34: }
   35: 
   36: #[derive(Clone, Debug, PartialEq, Eq)]
   37: pub struct AtomicPublicationSourceCertificate {
   38:     pub artifact_raw_sha256: String,
   39:     pub artifact_preprocessed_sha256: String,
   40:     /// Caller-declared label; the digests bind the actual source bytes.
   41:     pub source_revision: String,
   42:     /// Digest of every declared input/configuration field that can select the
   43:     /// preprocessed source or typed AST. The two artifact digests alone do not
   44:     /// bind frontend flags or the declared input identity.
   45:     pub analysis_configuration_sha256: String,
   46:     pub property: &'static str,
   47:     pub semantic_profile: &'static str,
   48:     pub scope: &'static str,
   49:     pub evidence_version: &'static str,
   50:     pub publisher: String,
   51:     pub consumer: String,
   52:     pub flag: String,
   53:     pub payload: String,
   54:     pub initial_value: i64,
   55:     pub published_value: i64,
   56:     /// The theorem applies only on the consumer branch that observed this value.
   57:     pub successful_observation: i64,
   58:     pub store_order: AtomicPublicationOrder,
   59:     pub load_order: AtomicPublicationOrder,
   60:     pub bounds: AtomicPublicationExecutionBounds,
   61:     pub assumptions: Vec<&'static str>,
   62: }
   63: 
   64: #[derive(Clone, Debug, PartialEq, Eq)]
   65: pub enum AtomicPublicationSourceRefusal {
   66:     InvalidSource(String),
   67:     InvalidProfile,
   68:     MissingOrAmbiguousDeclaration(&'static str),
   69:     RedeclaredProfileEntity(&'static str),
   70:     PlainFlagHasNoSynchronization,
   71:     VolatileFlagHasNoSynchronization,
   72:     UnsupportedAtomicFlag,
   73:     FlagMustBeStatic,
   74:     PayloadMustBeStaticInt,
   75:     UnsupportedInitializer,
   76:     UnsupportedPublisher,
   77:     UnsupportedConsumer,
   78:     UnsupportedHiddenAccess,
   79:     ExtraFlagAccess,
   80:     ExtraPayloadAccess,
   81:     ExtraEntryCall,
   82:     UnsupportedMemoryOrder,
   83:     UnsupportedExecutionBounds,
   84: }
   85: 
   86: pub fn extract_atomic_publication_source(
   87:     unit: &C11TranslationUnit<'_>,
   88:     source_revision: &str,
   89:     profile: &AtomicPublicationSourceProfile<'_>,
   90:     bounds: AtomicPublicationExecutionBounds,
   91: ) -> Result<AtomicPublicationSourceCertificate, AtomicPublicationSourceRefusal> {
   92:     if source_revision.is_empty()
   93:         || [
   94:             profile.publisher,
   95:             profile.consumer,
   96:             profile.flag,
   97:             profile.payload,
   98:         ]
   99:         .iter()
  100:         .any(|name| name.is_empty())
```

<a id="q36"></a>

## Q36 — `crates/qcore-c-gateway/tests/access_path_statements.rs:1–116`

File SHA-256: `57677fdb4f8f8e4c9e19f3fd7002e4387225291c65ba14a89e4d79f8b807cc75`

```rust
    1: use qcore_c_gateway::legacy_c::{
    2:     AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyInstruction, ResourceContracts,
    3:     UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
    4: };
    5: 
    6: fn run(source: &str) -> UnrestrictedLegacyResult {
    7:     analyze_unrestricted_c11(
    8:         &C11TranslationUnit {
    9:             source,
   10:             ..Default::default()
   11:         },
   12:         "subject",
   13:         &ResourceContracts::default(),
   14:     )
   15:     .expect("valid C")
   16: }
   17: 
   18: #[test]
   19: fn multiplication_is_not_a_pointer_access() {
   20:     let source = "int subject(void) { int x = 2; x = 3 * 4; return 0; }";
   21:     let result = run(source);
   22:     let UnrestrictedLegacyResult::Safe(certificate) = result else {
   23:         panic!("typed non-access should decide: {result:?}");
   24:     };
   25:     assert_eq!(certificate.summary.status, AnalysisStatus::Safe);
   26:     check_legacy_safety_certificate(&certificate).expect("independent rederivation");
   27: }
   28: 
   29: #[test]
   30: fn untracked_multi_pointer_declaration_is_not_an_access() {
   31:     let source = "struct S { int x; }; int subject(void *a, void *b) { const struct S *x = a, *y = b; return 0; }";
   32:     let result = run(source);
   33:     let UnrestrictedLegacyResult::Safe(certificate) = result else {
   34:         panic!("untracked pointer aliases should decide: {result:?}");
   35:     };
   36:     check_legacy_safety_certificate(&certificate).expect("independent rederivation");
   37: }
   38: 
   39: #[test]
   40: fn sizeof_vla_with_call_is_not_treated_as_unevaluated() {
   41:     let source = "int close(int); int subject(int n, int fd) { int x = sizeof(int[(close(fd), n)]); return x; }";
   42:     assert!(matches!(
   43:         run(source),
   44:         UnrestrictedLegacyResult::Unknown { .. }
   45:     ));
   46: }
   47: 
   48: #[test]
   49: fn embedded_and_indexed_paths_have_untracked_scope() {
   50:     let source = "struct I { int value; }; struct S { struct I inner; struct I *items; }; int subject(struct S *p, int i) { int x = p->inner.value + p->items[i].value; return 0; }";
   51:     let result = run(source);
   52:     let UnrestrictedLegacyResult::Safe(certificate) = result else {
   53:         panic!("untracked mixed path should decide: {result:?}");
   54:     };
   55:     check_legacy_safety_certificate(&certificate).expect("independent rederivation");
   56: }
   57: 
   58: #[test]
   59: fn pointer_store_and_union_still_refuse() {
   60:     for source in [
   61:         "struct S { int *p; }; int subject(struct S *s, int *q) { s->p = q; return 0; }",
   62:         "union U { int x; }; int subject(union U *u) { int x = u->x; return 0; }",
   63:         "typedef union U { int x; int y; } U; int subject(U *u) { int x = u->x; return 0; }",
   64:         "typedef union U { int x; int y; } U; typedef struct S { U *u; } S; int subject(S *s) { int x = s->u->x; return 0; }",
   65:         "typedef struct { int x; } A; struct H { A value; }; int subject(struct H *h) { h->value = h->value; return 0; }",
   66:     ] {
   67:         let result = run(source);
   68:         let UnrestrictedLegacyResult::Unknown {
   69:             partial: Some(summary),
   70:             ..
   71:         } = result
   72:         else {
   73:             panic!("unsupported path must stay Unknown: {result:?}");
   74:         };
   75:         assert!(
   76:             summary.functions["subject"]
   77:                 .diagnostics
   78:                 .iter()
   79:                 .any(|diagnostic| {
   80:                     matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description }
   81:                 if description == "indirect memory access is outside the resource projection")
   82:                 })
   83:         );
   84:     }
   85: }
   86: 
   87: #[test]
   88: fn extended_typed_member_chain_uses_untracked_scope() {
   89:     let source = "struct S { struct S *next; int value; }; int subject(struct S *p) { int x = p->next->next->next->next->value; return 0; }";
   90:     let result = run(source);
   91:     let UnrestrictedLegacyResult::Safe(certificate) = result else {
   92:         panic!("typed path should decide the chain: {result:?}");
   93:     };
   94:     check_legacy_safety_certificate(&certificate).expect("independent rederivation");
   95: }
   96: 
   97: #[test]
   98: fn pointer_value_read_binds_only_a_borrowed_alias() {
   99:     let source = "struct S { struct S *next; int *child; }; int subject(struct S *s) { int *q = s->next->next->next->next->child; return 0; }";
  100:     let result = run(source);
  101:     let UnrestrictedLegacyResult::Safe(certificate) = result else {
  102:         panic!("pointer read should bind a borrowed alias: {result:?}");
  103:     };
  104:     assert!(
  105:         certificate.summary.functions["subject"]
  106:             .cfg
  107:             .nodes
  108:             .iter()
  109:             .any(|node| {
  110:                 matches!(&node.instruction, LegacyInstruction::IndirectScalarAccess {
  111:             borrowed_destination: true, destination: Some(name), ..
  112:         } if name == "q")
  113:             })
  114:     );
  115:     check_legacy_safety_certificate(&certificate).expect("independent rederivation");
  116: }
```

<a id="q37"></a>

## Q37 — `crates/qcore-c-gateway/src/typed_position_shapes.rs:350–438`

File SHA-256: `14dc671b116eb270ce8b9fb63a9503dfc2ed18830d2da1ece21ecc5d7ebfabac`

```rust
  350: fn walk(index: &StatementIndex, node: &Value, out: &mut BTreeSet<&'static str>) {
  351:     match kind(node) {
  352:         "BinaryOperator" => match node["opcode"].as_str().unwrap_or("") {
  353:             "&&" | "||" => {
  354:                 out.insert("logical");
  355:             }
  356:             "," => {
  357:                 out.insert("comma");
  358:             }
  359:             "=" => {
  360:                 out.insert("assignment");
  361:             }
  362:             "+" | "-"
  363:                 if children(node)
  364:                     .iter()
  365:                     .any(|child| qual(child).is_some_and(|t| t.contains('*'))) =>
  366:             {
  367:                 out.insert("pointer-arithmetic");
  368:             }
  369:             _ => {}
  370:         },
  371:         "CompoundAssignOperator" => {
  372:             out.insert("assignment");
  373:         }
  374:         "UnaryOperator" => match node["opcode"].as_str().unwrap_or("") {
  375:             "&" => {
  376:                 out.insert("address");
  377:             }
  378:             "++" | "--" => {
  379:                 out.insert("increment");
  380:             }
  381:             _ => {}
  382:         },
  383:         "ConditionalOperator" | "BinaryConditionalOperator" => {
  384:             out.insert("conditional");
  385:         }
  386:         "CallExpr" => {
  387:             out.insert("call");
  388:         }
  389:         "CStyleCastExpr" => {
  390:             let scalar_to_scalar = is_scalar(node)
  391:                 && children(node)
  392:                     .first()
  393:                     .is_some_and(|inner| is_scalar(inner) || index.enums.is_enum(inner));
  394:             if !scalar_to_scalar {
  395:                 out.insert("cast");
  396:             }
  397:         }
  398:         "ImplicitCastExpr" => match node["castKind"].as_str().unwrap_or("") {
  399:             "LValueToRValue"
  400:             | "NoOp"
  401:             | "IntegralCast"
  402:             | "BitCast"
  403:             | "NullToPointer"
  404:             | "FunctionToPointerDecay" => {}
  405:             "ArrayToPointerDecay" => {
  406:                 out.insert("array-decay");
  407:             }
  408:             _ => {
  409:                 out.insert("implicit-conversion");
  410:             }
  411:         },
  412:         "StringLiteral" | "PredefinedExpr" => {
  413:             out.insert("string");
  414:         }
  415:         "StmtExpr" => {
  416:             out.insert("statement-expression");
  417:         }
  418:         "CompoundLiteralExpr" | "InitListExpr" => {
  419:             out.insert("aggregate-literal");
  420:         }
  421:         "UnaryExprOrTypeTraitExpr" => {
  422:             if !children(node).is_empty()
  423:                 || node["argType"]["qualType"]
  424:                     .as_str()
  425:                     .is_some_and(|ty| ty.contains('['))
  426:             {
  427:                 out.insert("sizeof-operand");
  428:             }
  429:             return;
  430:         }
  431:         "MemberExpr" => {
  432:             match node["referencedMemberDecl"]
  433:                 .as_str()
  434:                 .and_then(|id| index.fields.get(id))
  435:             {
  436:                 Some(Some(facts)) if facts.union_owner => {
  437:                     out.insert("union");
  438:                 }
```

<a id="q38"></a>

## Q38 — `crates/qcore-c-gateway/src/lexical_nested_assignment.rs:1–68`

File SHA-256: `f17986177b282437d77b2fa346feb424d644c58e1c2fe84a0f498c80dd4e2518`

```rust
    1: //! Card **W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT** (gateway rule 10o): every
    2: //! simple assignment of a body token run, wherever it stands.
    3: //!
    4: //! The lexical custody closure ([`crate::borrowed_actual::untracked_bases`])
    5: //! read one assignment per statement: the first top-level `=` of a run
    6: //! delimited by `;`, `{` and `}`. An assignment nested in a condition
    7: //! (`if (!(p = f()))`, `while ((p = next(q)))`), in a `for` clause
    8: //! (`for (p = f(); …)`, whose run starts at `for`), in a comma or `?:` operand,
    9: //! a later declarator (`T *a = f(), *b = a;`), a parenthesised destination
   10: //! (`(p) = f()`, the spelling of a macro-expanded assignment) and the value of
   11: //! a brace initialiser (`struct s v = { f() };`, split off by the `{`) were
   12: //! never read, so their destinations were called never bound although the
   13: //! body binds them.
   14: //!
   15: //! [`assignments`] enumerates them all. It is a **superset** reader: the
   16: //! caller keeps its own reading unchanged and adds these, so a name the old
   17: //! reading bound stays bound (monotone, `QCore.LexicalClosureNestedAssignment`).
   18: //!
   19: //! * **Operator.** A `=` token neither preceded by `=`, `!`, `<`, `>` or an
   20: //!   arithmetic/bitwise operator (a comparison or a compound assignment, whose
   21: //!   destination the closures already seed) nor followed by `=`.
   22: //! * **Destination.** The name immediately before the `=`, unless it is a
   23: //!   member (`.` or `->` before it: a sub-object store, which binds no
   24: //!   variable); or a parenthesised group before the `=` whose content, with
   25: //!   every enclosing pair stripped, is one name. Anything else (`a[i] =`,
   26: //!   `*p =`'s `p` is the name before the `=` and is read) names no destination.
   27: //! * **Value.** The tokens after the `=` up to the first `)` or `]` that closes
   28: //!   a group opened before it, or the first `;`, `{` or `}` outside every group
   29: //!   it opened: exactly the statement run the old reader read for the first
   30: //!   top-level `=`, and the rest of the enclosing group for a nested one. A
   31: //!   `,` does not end it: a later operand only makes the value more opaque, as
   32: //!   in the old reading. The one exception is a brace initialiser: a `{`
   33: //!   directly after the `=` is read through its matching `}`. A statement
   34: //!   expression value (`p = ({ … })`) is therefore read as its opening `(`
   35: //!   only, as before (rule 5l's `container_of` reading); its inner statements
   36: //!   are runs of their own and are read.
   37: //!
   38: //! Every function here is a pure reader of the analyzer's own token texts, so
   39: //! W4 derivation, W4 validation and certificate rederivation repeat it from
   40: //! the certificate's own content.
   41: 
   42: use std::collections::BTreeSet;
   43: use std::ops::Range;
   44: 
   45: use crate::typed_field_access::is_name;
   46: 
   47: /// Heads that make a following `=` a comparison or a compound assignment.
   48: const COMPOUND_HEADS: &[&str] = &["=", "!", "<", ">", "+", "-", "*", "/", "%", "&", "|", "^"];
   49: 
   50: /// One simple assignment of a body.
   51: #[derive(Clone, Debug, PartialEq, Eq)]
   52: pub(crate) struct Assignment<'a> {
   53:     /// The destination name.
   54:     pub(crate) destination: &'a str,
   55:     /// The value's token range in the body.
   56:     pub(crate) value: Range<usize>,
   57:     /// Whether the destination was spelled in parentheses (`(p) = …`).
   58:     pub(crate) parenthesised: bool,
   59:     /// The index of the `=` token in the body.
   60:     pub(crate) operator: usize,
   61: }
   62: 
   63: /// Whether the `=` at `index` is a simple assignment operator.
   64: fn simple_operator(body: &[&str], index: usize) -> bool {
   65:     body[index] == "="
   66:         && body.get(index + 1) != Some(&"=")
   67:         && index
   68:             .checked_sub(1)
```

<a id="q39"></a>

## Q39 — `crates/qcore-c-gateway/src/typed_root_declarations.rs:280–425`

File SHA-256: `c7cf045fb8163cfcc82d8e31bd4079fa8de795796ff18d213ab3ebcbc6b69ef9`

```rust
  280:     fn mark(&mut self, name: &str, reason: &str) {
  281:         if self.risky.insert(name.to_owned()) {
  282:             self.reasons.insert(name.to_owned(), reason.to_owned());
  283:         }
  284:     }
  285: 
  286:     fn declare(
  287:         &mut self,
  288:         node: &Value,
  289:         kind: DeclarationKind,
  290:         enums: &EnumFacts,
  291:         ids: &mut BTreeSet<String>,
  292:     ) {
  293:         let Some(name) = node["name"].as_str() else {
  294:             return;
  295:         };
  296:         if let Some(id) = node["id"].as_str() {
  297:             ids.insert(id.to_owned());
  298:         }
  299:         let typed = is_scalar(node) || enums.is_enum(node) || is_pointer(node);
  300:         self.declarations
  301:             .entry(name.to_owned())
  302:             .or_default()
  303:             .push(Declaration { kind, typed });
  304:     }
  305: }
  306: 
  307: struct Walk<'a, 'f> {
  308:     enums: &'a EnumFacts,
  309:     contracts: &'a ResourceContracts,
  310:     variadic_tails: &'a BTreeSet<String>,
  311:     facts: &'f mut Facts,
  312:     ids: &'f mut BTreeSet<String>,
  313:     /// `(destination, values, place)` of every plain assignment and
  314:     /// initialiser; `place` says where it stands (parity only).
  315:     assignments: Vec<(String, Vec<&'a Value>, &'static str)>,
  316:     /// `(spelling, id)` of every variable reference.
  317:     references: Vec<(String, String)>,
  318: }
  319: 
  320: impl<'a> Walk<'a, '_> {
  321:     fn bind(&mut self, name: &str, reason: &'static str) {
  322:         self.facts.mark(name, reason);
  323:         self.facts.bound.insert(name.to_owned());
  324:     }
  325: 
  326:     fn node(&mut self, node: &'a Value) -> Result<(), Refusal> {
  327:         self.node_at(node, false)
  328:     }
  329: 
  330:     /// `statement`: `node` is a whole statement of a block, a `case` or a
  331:     /// label (parity only: where an assignment stands).
  332:     fn node_at(&mut self, node: &'a Value, statement: bool) -> Result<(), Refusal> {
  333:         match kind(node) {
  334:             "GCCAsmStmt" | "MSAsmStmt" => return Err(Refusal::InlineAssembly),
  335:             "BlockExpr" => return Err(Refusal::Block),
  336:             "TypedefDecl" => return Err(Refusal::BlockScopeTypedef),
  337:             "VarDecl" => {
  338:                 let storage = node["storageClass"].as_str();
  339:                 let declaration = if storage.is_none() {
  340:                     DeclarationKind::Automatic
  341:                 } else {
  342:                     DeclarationKind::Other
  343:                 };
  344:                 self.facts.declare(node, declaration, self.enums, self.ids);
  345:                 if let Some(name) = node["name"].as_str() {
  346:                     let values = children(node).iter().collect::<Vec<_>>();
  347:                     if !values.is_empty() {
  348:                         self.facts.bound.insert(name.to_owned());
  349:                         self.assignments
  350:                             .push((name.to_owned(), values, "initialiser"));
  351:                     }
  352:                 }
  353:             }
  354:             "DeclRefExpr" => {
  355:                 if is_object(node)
  356:                     && let (Some(name), Some(id)) = (
  357:                         node["referencedDecl"]["name"].as_str(),
  358:                         node["referencedDecl"]["id"].as_str(),
  359:                     )
  360:                 {
  361:                     self.references.push((name.to_owned(), id.to_owned()));
  362:                 }
  363:                 if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
  364:                     && node["referencedDecl"]["isImplicit"].as_bool() == Some(true)
  365:                 {
  366:                     return Err(Refusal::ImplicitCallee);
  367:                 }
  368:             }
  369:             "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
  370:                 let pair = children(node);
  371:                 if let [left, right] = pair {
  372:                     match strip_lvalue(left) {
  373:                         Some(target) if kind(target) == "DeclRefExpr" && is_object(target) => {
  374:                             if let Some(name) = target["referencedDecl"]["name"].as_str() {
  375:                                 self.facts.bound.insert(name.to_owned());
  376:                                 let place = if statement { "statement" } else { "nested" };
  377:                                 self.assignments.push((name.to_owned(), vec![right], place));
  378:                             }
  379:                         }
  380:                         _ => {
  381:                             if let Some(root) = object_root(left) {
  382:                                 self.bind(root, "sub-object-store");
  383:                             }
  384:                         }
  385:                     }
  386:                 }
  387:             }
  388:             "CompoundAssignOperator" => {
  389:                 if let Some(root) = children(node).first().and_then(object_root) {
  390:                     self.bind(root, "compound-assignment");
  391:                 }
  392:             }
  393:             "UnaryOperator"
  394:                 if matches!(node["opcode"].as_str(), Some("++") | Some("--") | Some("&")) =>
  395:             {
  396:                 let reason = if node["opcode"].as_str() == Some("&") {
  397:                     "address"
  398:                 } else {
  399:                     "increment"
  400:                 };
  401:                 if let Some(root) = children(node).first().and_then(object_root) {
  402:                     self.bind(root, reason);
  403:                 }
  404:             }
  405:             "CallExpr" => self.call(node),
  406:             // Builtins Clang does not spell as a call (`__atomic_*`,
  407:             // `__c11_atomic_*`, `__builtin_va_arg`) take every operand as an
  408:             // uncontracted callee would.
  409:             "AtomicExpr" | "VAArgExpr" => {
  410:                 for operand in children(node) {
  411:                     self.uncontracted_operand(operand);
  412:                 }
  413:             }
  414:             _ => {}
  415:         }
  416:         let block = matches!(
  417:             kind(node),
  418:             "CompoundStmt" | "CaseStmt" | "DefaultStmt" | "LabelStmt"
  419:         );
  420:         for child in children(node) {
  421:             self.node_at(child, block)?;
  422:         }
  423:         Ok(())
  424:     }
  425: 
```

<a id="q40"></a>

## Q40 — `crates/qcore-c-gateway/src/legacy_return_family.rs:1–101`

File SHA-256: `c6e5650d61d77177ee2c389f6675316f7afe75ffc113448ccc199524e2a525f0`

```rust
    1: //! Card **W45-RETURN-EXPRESSION-FAMILIES**: two families of `return`
    2: //! expression the position used to refuse ("unmodeled call, aggregate or
    3: //! indirect access in return expression"), each lowered to what C evaluates
    4: //! with the returned value's custody stated exactly.
    5: //!
    6: //! Both are consulted only from `Parser::return_expression_instruction`, after
    7: //! every older route — the strict and pointer-leaf rules (constraint 5b), the
    8: //! cast return (W44-LEAK-PIN-AUDIT), the W4 whole call, both B01 routes and
    9: //! W43's refused-whole-call lowering — has declined, so no expression an older
   10: //! route decides is decided by different code.
   11: //!
   12: //! # S — the scalar-result read
   13: //!
   14: //! `return E;` in a function whose **definition declares a scalar value
   15: //! result** (an arithmetic or enumeration type, or a typedef the unit's
   16: //! declaration reader resolves to one, with no pointer level:
   17: //! [`TypedAccessModel::is_scalar_value_type`]). C converts `E` to that type
   18: //! (C11 6.8.6.4p3), so the returned value is a number: **it carries no
   19: //! custody**, and the exit escapes nothing (`NoOp`). What remains is the
   20: //! evaluation of `E`, and the rule admits only an evaluation that is a *read*:
   21: //!
   22: //! * no call (a call-shaped `NAME (` of any kind refuses, `_Generic` and the
   23: //!   builtins included), no brace, no assignment, no increment or decrement,
   24: //!   no comma, no GNU `?:` and no unary `&`;
   25: //! * a type operator (`sizeof`, `_Alignof`) only over a parenthesised type
   26: //!   name of identifiers the reader resolves and that is never variably
   27: //!   modified ([`TypedAccessModel::never_variably_modified`]), so the operand
   28: //!   is not evaluated (C11 6.5.3.4p2). A pointer type name (`sizeof(void *)`)
   29: //!   keeps the fixed type-query rule's refusal, which its own pins hold;
   30: //! * no name that may hold a tracked descriptor is mentioned: an integer
   31: //!   identity is custody, never data;
   32: //! * every name `E` evaluates is **untracked** — one this body can never bind
   33: //!   to a tracked identity (`UntrackedBases::never_bound`, card C02-P premise
   34: //!   4, the reading the chain route's root obligation uses) — or a declared
   35: //!   scalar. A read through an untracked root reads an object no reaching
   36: //!   path bound, and a member of such an object is a place this function never
   37: //!   names (C02-P premise 3), so the links of a chain below it carry no
   38: //!   obligation. A directly dereferenced untracked root is still a base of the
   39: //!   read node, so the flow check stands behind the syntactic premise: such a
   40: //!   name can never be released by this body, so the check can refuse
   41: //!   (`unknown_alias` on a possibly-null entry binding) but never accuse;
   42: //! * a name the body **may** bind is admitted only as the root of exactly one
   43: //!   dereference — `B -> F`, `B -> F . G`, `B [ I ]`, `* B` — that C evaluates
   44: //!   unconditionally (not after a `?`, `&&` or `||` of its group). It is a
   45: //!   **base** of the read node and keeps the typed access route's obligation
   46: //!   in transfer: unbound, a non-null untracked binding, or exactly one known,
   47: //!   non-null, live identity, else `unknown_alias`; a base settled on a
   48: //!   released identity is a `use_after_release` candidate. The position
   49: //!   premise is what keeps that accusation to paths C actually takes. Any
   50: //!   other use of such a name — as a value, under a second dereference,
   51: //!   inside a parenthesised operand — refuses.
   52: //!
   53: //! The read node is `IndirectScalarAccess` with no destination, no frame and
   54: //! no chain: it changes no binding, status, marker or link, and adds only the
   55: //! bases' obligations and the C02-P-ALIAS obligations of every `->` root. It
   56: //! is not a new instruction, so `descriptor_custody::admits_descriptor_read`
   57: //! needs no new arm (and no descriptor reaches it).
   58: //!
   59: //! A **pointer-valued** result is not this rule: `p->buf` may be a load, an
   60: //! array member's decay or, as `&p->f`, an interior address, which is a result
   61: //! derived from `p`; telling them apart needs member typing, and publishing a
   62: //! derived pointer as a result that carries nothing would let a caller free
   63: //! `p` and keep using the result unaccused. It keeps the refusal.
   64: //!
   65: //! # K — a cast of a whole call
   66: //!
   67: //! `return ( T1 ) … ( Tn ) CALL ( ARGS );`, possibly parenthesised, with every
   68: //! `Ti` a type name the reader resolves (so `(f)(x)`, a call of `f`, is never
   69: //! read as a cast). The synthetic `PLACE = CALL ( ARGS );` is classified
   70: //! exactly as card C-GATEWAY-RETURN-CALL-W4 classifies an uncast whole call:
   71: //!
   72: //! * a **refusal** is lowered to that refusal and a `NoOp` exit — card
   73: //!   W43-RETURN-EXPRESSION's re-attribution, now under a cast. It decides
   74: //!   nothing; the site counter is restored;
   75: //! * an admitted call whose result is **`None`** carries nothing whatever the
   76: //!   casts convert it to, and is taken as W4 takes it;
   77: //! * an admitted **`Owned`** result is taken only when every cast targets a
   78: //!   pointer type and the unit's own declaration of `CALL` returns an object
   79: //!   pointer (`declared_abi::declares_object_pointer_result`): a
   80: //!   pointer-to-pointer conversion designates the same object (C11
   81: //!   6.3.2.3p7), so the exit escapes exactly the site the call created;
   82: //! * with **no** cast (only parentheses) the answer is W4's own admission,
   83: //!   derived results under `admit_derived_returns` included;
   84: //! * every other classification — a `Reallocated` or `Derived` result under a
   85: //!   cast, an `Owned` result under an integer cast, anything that is not the
   86: //!   call into `PLACE` — keeps the return refusal, with the site counter
   87: //!   restored.
   88: //!
   89: //! # Evaluation order
   90: //!
   91: //! Both families lower to one evaluation node before the exit node, at the
   92: //! `return`'s own line, so every call and read of the expression happens
   93: //! before the function returns, as C sequences it.
   94: //!
   95: //! The Lean model is `QCore.ReturnExpressionFamilies`; as for every gateway
   96: //! module, the correspondence is maintained by review (constraint 28).
   97: 
   98: use qcore_schemas::contract::ResourceReturnMode;
   99: 
  100: use super::{LegacyInstruction, Parser, Token, descriptor_custody, return_call};
  101: use crate::typed_field_access::TypedAccessModel;
```

<a id="q41"></a>

## Q41 — `crates/qcore-c-gateway/src/loop_owner_property_keys.rs:1–95`

File SHA-256: `12886a9aae73eb98ad3e131f9959926c1169454e452e6e05c3466c092cd774be`

```rust
    1: //! The keys a variadic property setter carries (card
    2: //! **W47-SERVICE-PROPERTY-KEYS**, gateway rule 10p's amendment).
    3: //!
    4: //! Step B's list-head region (card W47-LOOP-OWNER-INSTANCE-FLOW) counts every
    5: //! call whose closure reaches a hazard. A setter such as
    6: //! `flb_service_set(ctx, "Flush", "1", …, NULL)` hands each key to a property
    7: //! function that dispatches on the key's text, and only some keys reach a
    8: //! hazard (`Plugins_File` registers plugins into `config->in_plugins`). This
    9: //! module reads such a call as reaching only its keys' effects, when every
   10: //! part of that reading is derived from the source:
   11: //!
   12: //! 1. **The call's arguments** (typed reading): one argument list recorded at
   13: //!    the call's line, NULL-terminated (the integer constant `0` converted to
   14: //!    a pointer), an even number of key/value arguments before it, no other
   15: //!    null argument, every key a string literal of printable ASCII without a
   16: //!    backslash or quote. Values are not read: no key's reach depends on its
   17: //!    value, because every branch is read whole (a file-path handler's whole
   18: //!    closure, whatever file the value names).
   19: //! 2. **The setter's body**: `va_start(VA, LAST)` on its last named
   20: //!    parameter; exactly two `va_arg(VA, char *)`, the first the loop
   21: //!    condition `while ((KEY = va_arg(…)))`, the second the loop body's first
   22: //!    statement `VALUE = va_arg(…);`; `KEY` and `VALUE` locals assigned there
   23: //!    only, their addresses never taken; `KEY` passed, as one whole argument,
   24: //!    to exactly one call, the property function `P`, inside that loop; no
   25: //!    jump. So `P` sees exactly the keys at even variadic positions, in order,
   26: //!    up to the first null key or value.
   27: //! 3. **The key table** (both readings, cross-checked): `P` walks one table
   28: //!    (`char *KV = TABLE[0].KF; while (KV != NULL) { if (CMP(KV, K, LEN) ==
   29: //!    0) { MATCHED } KV = TABLE[++I].KF; }`, `LEN = strnlen(K, N)`, `KV`,
   30: //!    `I`, `LEN` assigned nowhere else). The typed reading gives the table's
   31: //!    elements under the build (keys after macro expansion, the type
   32: //!    enumerator); the configured lexical reading gives each element's
   33: //!    `offsetof(struct R, FIELD)`; the two must agree element for element
   34: //!    (count, enumerator), the last key being the only null one.
   35: //! 4. **The comparison**, from `CMP`'s body: exactly `size_t L; L =
   36: //!    strnlen(A, N1); if (strncasecmp(A, B, C) == 0 && L == C) { return 0; }
   37: //!    return -1;`. A table key `e` matches a key `k` exactly when, with `n =
   38: //!    min(|k|, N)`, the first `n` bytes agree ignoring ASCII case and
   39: //!    `min(|e|, N1) = n`. Every key matched by no entry keeps every branch.
   40: //! 5. **The branch chain** in `MATCHED`: one `if (!strncasecmp(KV, C, M))
   41: //!    {…} else if (…) {…} … else {…}` chain, every arm a block, each
   42: //!    constant `C` read by the typed reading. An entry takes the first arm
   43: //!    whose comparison its key satisfies, else the final `else`. Everything
   44: //!    in `P` outside the chain's arms is common to every key.
   45: //! 6. **The scalar store** (the final `else`): `switch (TABLE[I].TYPEF)`, each
   46: //!    case's stores `*(T *)((char *)CONFIG + TABLE[I].OFFF)`. An entry that
   47: //!    reaches it is admitted only when its offset names a field of the
   48: //!    property function's own config record other than a listed list member,
   49: //!    declared once in that record with exactly the store's type (`unsigned`
   50: //!    and `signed` ignored), so the store stays inside that field; and no
   51: //!    such pointer arithmetic stands anywhere else in `P`.
   52: //!
   53: //! The call then reaches the functions named (called, taken as a value, or
   54: //! named by an invoked macro's replacement list) in the setter's body and in
   55: //! `P`'s body outside the arms no key selects; the setter and `P` are read in
   56: //! place (their own hazards, and their unmodelled calls, count). Anything
   57: //! else — no typed list, a non-literal key, an odd or unterminated list, an
   58: //! unknown key, an entry this reading cannot classify, a shape that is not
   59: //! the one read, a conditional directive left undecided inside a read body,
   60: //! a field dispatch — keeps the whole call, every branch.
   61: //!
   62: //! Premises stated, not proved (trust boundary 28, beside rule 10p's): the
   63: //! typed reading's argument and initializer extraction; the C library's
   64: //! `strncasecmp` folds the printable ASCII bytes of such keys no more
   65: //! permissively than ASCII case folding (P-KEY-FOLD); `strnlen` as C
   66: //! specifies it; and `va_arg` reads the variadic arguments in order.
   67: 
   68: use std::collections::{BTreeMap, BTreeSet};
   69: 
   70: use super::StepB;
   71: use super::loop_owner_instance::{
   72:     NOT_CALLS, address_taken, assignments, call_arguments, declared_local, ordered_parameters,
   73: };
   74: use crate::legacy_c::is_identifier;
   75: use crate::typed_dispatch_census::{ArgumentCall, literal_call_key};
   76: 
   77: /// Which calls this reading applies to, and what the typed reading records
   78: /// for it. Nothing here states an effect: the setters' bodies, the property
   79: /// function, its table and its comparison are read from the source.
   80: #[derive(Clone, Copy, Debug)]
   81: pub struct PropertyKeyTables {
   82:     /// `(setter, index of its first variadic argument)`.
   83:     pub setters: &'static [(&'static str, usize)],
   84:     /// Property functions whose keyed-branch comparisons the typed reading
   85:     /// records (a setter's property function must be one of them).
   86:     pub property_functions: &'static [&'static str],
   87:     /// The comparison a keyed branch and the comparison function call.
   88:     pub comparison: &'static str,
   89:     /// The bound a key's length is read with.
   90:     pub bound: &'static str,
   91:     /// Key tables whose initializers the typed reading records.
   92:     pub key_tables: &'static [&'static str],
   93: }
   94: 
   95: /// Fluent Bit at the `90978ac6` export: `src/flb_lib.c` (`flb_service_set`
```

<a id="q42"></a>

## Q42 — `crates/qcore-c-gateway/src/cleanup_replay_source.rs:145–206`

File SHA-256: `e49dc0de7877db377e53833e5e097ebeae50684b70a5bbaab0c98fdfce1227d1`

```rust
  145:             value: Box::new(self::value(value)?),
  146:         },
  147:         SourceExpression::Binary {
  148:             operator,
  149:             left,
  150:             right,
  151:         } => CleanupReplayExpressionRecord::Binary {
  152:             operator: operator.clone(),
  153:             left: Box::new(self::value(left)?),
  154:             right: Box::new(self::value(right)?),
  155:         },
  156:         SourceExpression::LocalStorage { .. } => {
  157:             return Err(
  158:                 "cleanup replay projection does not represent address-taken stack slots".into(),
  159:             );
  160:         }
  161:         SourceExpression::LoopIndex { .. } => {
  162:             return Err(
  163:                 "cleanup replay projection does not represent loop induction values".into(),
  164:             );
  165:         }
  166:         SourceExpression::SizeOf { .. } => {
  167:             return Err("cleanup replay projection does not represent sizeof operands".into());
  168:         }
  169:         SourceExpression::Aggregate { .. } => {
  170:             return Err(
  171:                 "cleanup replay projection does not represent aggregate initializers".into(),
  172:             );
  173:         }
  174:         SourceExpression::Index { .. } => {
  175:             return Err("cleanup replay projection does not represent subscripts".into());
  176:         }
  177:         SourceExpression::Function { .. } => {
  178:             return Err("cleanup replay projection does not represent callee designators".into());
  179:         }
  180:     };
  181:     Ok(CleanupReplayValueRecord {
  182:         c_type: value.c_type.clone(),
  183:         expression,
  184:     })
  185: }
  186: 
  187: /// Project every complete path of one summary. Scan loops, terminations, bare
  188: /// returns and every callee outside the profile's own three declared names
  189: /// stay outside the vocabulary, so their functions keep the existing
  190: /// `Unknown`.
  191: fn project(
  192:     summary: &SourceSummary,
  193:     profile: &CleanupProfileRecord,
  194: ) -> Result<CleanupReplayFunctionRecord, String> {
  195:     if summary.has_scan_loop() {
  196:         return Err("cleanup replay projection does not represent scan loops".into());
  197:     }
  198:     if summary.paths().is_empty() {
  199:         return Err("cleanup replay projection requires at least one complete source path".into());
  200:     }
  201:     // The closed callee table is the profile's own declared names, so it is
  202:     // contract-derived rather than a caller assertion.
  203:     let admitted = [
  204:         profile.syscall.as_str(),
  205:         profile.errno_function.as_str(),
  206:         profile.report_function.as_str(),
```

<a id="q43"></a>

## Q43 — `crates/qcore-c-gateway/src/typed_access_path.rs:1220–1348`

File SHA-256: `dddd56617d0860536214ed79a2aa413a25e0ed84e6b100dbd3d19012beeaa792`

```rust
 1220: 
 1221:     fn plan(source: &str, statement: &str) -> Option<StatementPlan> {
 1222:         let _scope = enter_source(source, CCompilerDialect::C11, None);
 1223:         let index = StatementIndex::read("f")?;
 1224:         let wanted = tokenize(statement);
 1225:         let texts = wanted
 1226:             .iter()
 1227:             .map(|token| token.text.as_str())
 1228:             .collect::<Vec<_>>();
 1229:         index.statement(&texts, 1, |_| true)
 1230:     }
 1231: 
 1232:     #[test]
 1233:     fn typed_scalar_chain_names_each_obligation() {
 1234:         let source = "struct S { struct S *next; int value; }; int f(struct S *p) { int x = p->next->value; return 0; }";
 1235:         let plan = plan(source, "int x = p->next->value").expect("typed chain");
 1236:         assert_eq!(plan.scalar.bases, ["p"]);
 1237:         assert_eq!(plan.chains.len(), 1);
 1238:         assert_eq!(plan.chains[0].links, ["next"]);
 1239:         assert_eq!(plan.scalar.destination.as_deref(), Some("x"));
 1240:     }
 1241: 
 1242:     #[test]
 1243:     fn unsupported_pointer_store_union_cast_and_call_refuse() {
 1244:         let cases = [
 1245:             (
 1246:                 "struct S { int *next; }; int f(struct S *p, int *q) { p->next = q; return 0; }",
 1247:                 "p->next = q",
 1248:             ),
 1249:             (
 1250:                 "union U { int value; }; int f(union U *p) { int x = p->value; return 0; }",
 1251:                 "int x = p->value",
 1252:             ),
 1253:             (
 1254:                 "int f(int *p) { int x = *(int *)p; return 0; }",
 1255:                 "int x = *(int *)p",
 1256:             ),
 1257:             (
 1258:                 "int g(void); int f(void) { int x = g(); return 0; }",
 1259:                 "int x = g()",
 1260:             ),
 1261:         ];
 1262:         for (source, statement) in cases {
 1263:             assert!(plan(source, statement).is_none(), "{statement}");
 1264:         }
 1265:     }
 1266: 
 1267:     #[test]
 1268:     fn for_init_is_a_statement_but_the_condition_is_not() {
 1269:         let source = "struct S { int value; }; int f(struct S *p) { for (int x = p->value; x < 1; x = x + 1) { } return 0; }";
 1270:         assert!(plan(source, "int x = p->value").is_some());
 1271:         assert!(plan(source, "x < 1").is_none());
 1272:     }
 1273: 
 1274:     #[test]
 1275:     fn untracked_multi_pointer_declaration_has_no_access() {
 1276:         let source = "struct S { int x; }; int f(void *a, void *b) { const struct S *x = a, *y = b; return 0; }";
 1277:         let _scope = enter_source(source, CCompilerDialect::C11, None);
 1278:         let index = StatementIndex::read("f").expect("typed function");
 1279:         let tokens = tokenize("const struct S *x = a, *y = b");
 1280:         let texts = tokens
 1281:             .iter()
 1282:             .map(|token| token.text.as_str())
 1283:             .collect::<Vec<_>>();
 1284:         assert_eq!(index.statement(&texts, 1, |_| true).unwrap().accesses, 0);
 1285:         assert!(index.statement(&texts, 1, |name| name != "a").is_none());
 1286:     }
 1287: 
 1288:     #[test]
 1289:     fn sizeof_vla_bound_may_execute_and_refuses() {
 1290:         let source = "int close(int); int f(int n, int fd) { int x = sizeof(int[(close(fd), n)]); return x; }";
 1291:         assert!(plan(source, "int x = sizeof(int[(close(fd), n)])").is_none());
 1292:         let static_source = "int f(void) { int x = sizeof(int *); return x; }";
 1293:         assert!(plan(static_source, "int x = sizeof(int *)").is_some());
 1294:     }
 1295: 
 1296:     #[test]
 1297:     fn mixed_embedded_and_indexed_path_needs_untracked_root() {
 1298:         let source = "struct I { int value; }; struct S { struct I inner; struct I *items; }; int f(struct S *p, int i) { int x = p->inner.value + p->items[i].value; return 0; }";
 1299:         let _scope = enter_source(source, CCompilerDialect::C11, None);
 1300:         let index = StatementIndex::read("f").expect("typed function");
 1301:         let tokens = tokenize("int x = p->inner.value + p->items[i].value");
 1302:         let texts = tokens
 1303:             .iter()
 1304:             .map(|token| token.text.as_str())
 1305:             .collect::<Vec<_>>();
 1306:         let plan = index
 1307:             .statement(&texts, 1, |_| true)
 1308:             .expect("untracked path");
 1309:         assert_eq!(plan.accesses, 2);
 1310:         assert!(plan.chains.iter().all(|chain| chain.links.is_empty()));
 1311:         assert!(index.statement(&texts, 1, |name| name != "p").is_none());
 1312:     }
 1313: 
 1314:     #[test]
 1315:     fn field_owner_refuses_typedef_union_but_accepts_typedef_struct() {
 1316:         let union = "typedef union U { int x; int y; } U; int f(U *u) { int x = u->x; return 0; }";
 1317:         assert!(plan(union, "int x = u->x").is_none());
 1318:         let nested = "typedef union U { int x; int y; } U; typedef struct S { U *u; } S; int f(S *s) { int x = s->u->x; return 0; }";
 1319:         assert!(plan(nested, "int x = s->u->x").is_none());
 1320:         let accepted = "typedef struct S { int x; } S; int f(S *s) { int x = s->x; return 0; }";
 1321:         assert!(plan(accepted, "int x = s->x").is_some());
 1322:     }
 1323: 
 1324:     #[test]
 1325:     fn duplicate_same_line_refuses_but_distinct_lines_keep_their_candidates() {
 1326:         let repeated = "struct S { int x; }; int f(struct S *p) { p->x = 1; p->x = 1; return 0; }";
 1327:         let _scope = enter_source(repeated, CCompilerDialect::C11, None);
 1328:         let index = StatementIndex::read("f").expect("typed function");
 1329:         let tokens = tokenize("p->x = 1");
 1330:         let texts = tokens
 1331:             .iter()
 1332:             .map(|token| token.text.as_str())
 1333:             .collect::<Vec<_>>();
 1334:         assert!(index.statement(&texts, 1, |_| true).is_none());
 1335: 
 1336:         let separated =
 1337:             "struct S { int x; }; int f(struct S *p) {\n p->x = 1;\n p->x = 1;\n return 0; }";
 1338:         let _scope = enter_source(separated, CCompilerDialect::C11, None);
 1339:         let index = StatementIndex::read("f").expect("typed function");
 1340:         assert!(index.statement(&texts, 2, |_| true).is_some());
 1341:         assert!(index.statement(&texts, 3, |_| true).is_some());
 1342:         assert!(index.statement(&texts, 1, |_| true).is_none());
 1343:     }
 1344: 
 1345:     #[test]
 1346:     fn nested_control_candidates_and_function_ast_are_reused() {
 1347:         let source = "struct S { int x; }; int f(struct S *p) {\n if (1) {\n p->x = 1;\n } else {\n p->x = 2;\n }\n return 0; }";
 1348:         let _scope = enter_source(source, CCompilerDialect::C11, None);
```

<a id="q44"></a>

## Q44 — `crates/qcore-c-gateway/src/typed_field_access.rs:309–350`

File SHA-256: `3733e3d1c362e993ca3a94acc0b914ebedb59302bcfa250492054e76998ee0af`

```rust
  309: /// Arithmetic type keywords. `void`, `_Complex` and `_Imaginary` are absent by
  310: /// decision: they are not admitted as scalar field or operand types.
  311: const ARITHMETIC_KEYWORDS: &[&str] = &[
  312:     "char", "short", "int", "long", "signed", "unsigned", "float", "double", "_Bool",
  313: ];
  314: 
  315: /// Qualifiers and storage classes that do not change the admitted class.
  316: /// `volatile` and `_Atomic` are deliberately absent: they reject.
  317: const IGNORED_SPECIFIERS: &[&str] = &[
  318:     "const",
  319:     "restrict",
  320:     "__restrict",
  321:     "__restrict__",
  322:     "static",
  323:     "register",
  324:     "extern",
  325:     "auto",
  326:     "inline",
  327:     "_Noreturn",
  328:     "_Thread_local",
  329:     "__extension__",
  330:     "__inline",
  331:     "__inline__",
  332: ];
  333: 
  334: const REJECTING_QUALIFIERS: &[&str] = &["volatile", "_Atomic"];
  335: 
  336: /// Declaration suffixes that carry no type and may stand between a
  337: /// declarator's parameter list and its `;` or body.
  338: const DECLARATION_SUFFIXES: &[&str] = &["__attribute__", "__attribute", "__asm__", "asm", "__asm"];
  339: 
  340: /// Keywords that can never name a declarator.
  341: const RESERVED_WORDS: &[&str] = &[
  342:     "auto",
  343:     "break",
  344:     "case",
  345:     "char",
  346:     "const",
  347:     "continue",
  348:     "default",
  349:     "do",
  350:     "double",
```

<a id="q45"></a>

## Q45 — `crates/qcore-c-gateway/src/legacy_c.rs:1–85`

File SHA-256: `37944d1ddd09ff9edcdd5cebb6807166f0e489e91771cf7f2948e253e49ef9a5`

```rust
    1: //! Conservative resource analysis for the Cycle 9 annotated legacy-C subset.
    2: //!
    3: //! The parser intentionally accepts more C than the analyzer models.  Every
    4: //! construct outside the supported structured subset becomes an `Unknown`
    5: //! instruction. `Safe` concerns the supported custody projection under its
    6: //! declared resource contracts, not arbitrary C memory safety. Indirect reads,
    7: //! aggregate/indexed accesses and ambiguous expression forms without a model
    8: //! remain unsupported; symbolic resource place spelling is not access proof.
    9: 
   10: #[path = "legacy_joined_provenance.rs"]
   11: pub(crate) mod joined_provenance;
   12: 
   13: #[path = "legacy_out_alias.rs"]
   14: mod out_alias;
   15: 
   16: #[path = "legacy_return_call.rs"]
   17: pub(crate) mod return_call;
   18: 
   19: #[path = "legacy_enumeration.rs"]
   20: pub mod enumeration;
   21: 
   22: pub use enumeration::{
   23:     EnumerationRefusal, SourceDefinitionInventory, UnanalyzableDefinition, UnanalyzableForm,
   24:     enumerate_source_definitions,
   25: };
   26: 
   27: #[path = "legacy_location.rs"]
   28: pub mod location;
   29: 
   30: pub use location::{
   31:     LocationRefusal, OriginFile, OriginalLocation, SourceLineMap, map_marked_preprocessing,
   32:     map_preprocessed_lines, unlocated_site,
   33: };
   34: 
   35: #[path = "legacy_preparation.rs"]
   36: mod preparation;
   37: 
   38: pub use preparation::{
   39:     PreparationMetrics, PreparationStageMetrics, PreparationStageOutcome, PreparedTranslationUnit,
   40:     prepare_translation_unit,
   41: };
   42: 
   43: #[path = "legacy_recursion.rs"]
   44: pub(crate) mod recursion;
   45: 
   46: #[path = "legacy_literal_premise.rs"]
   47: pub(crate) mod literal_premise;
   48: 
   49: #[path = "legacy_declared_abi.rs"]
   50: pub mod declared_abi;
   51: 
   52: pub use declared_abi::{AbiAgreementRow, AbiOutcome, AbiRefusal, declared_abi_census};
   53: 
   54: #[path = "legacy_declared_modes.rs"]
   55: pub mod declared_modes;
   56: 
   57: // Card W45-CONTRACT-DEFERRED-REVIEW: a declared contract awaiting human review
   58: // is withheld and its call sites are held. Everything the card adds lives
   59: // there; this file only carries the hooks marked with the card's name.
   60: #[path = "legacy_review_hold.rs"]
   61: pub mod review_hold;
   62: 
   63: pub use review_hold::{AWAITING_HUMAN_REVIEW, held_call, hold_description};
   64: 
   65: // Card W45-CONTRACT-REVISION-MODE: a pinned contract the audit withheld for a
   66: // pin mismatch; its call sites are held exactly as a pending review's are.
   67: #[path = "legacy_pin_hold.rs"]
   68: pub mod pin_hold;
   69: 
   70: // Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: a `Pending` row is joined
   71: // with every reason its function would carry with the held declarations in
   72: // force, so a held call never masks another reason.
   73: #[path = "legacy_review_standing.rs"]
   74: pub mod review_standing;
   75: 
   76: // Card W45-REFUTED-CONTRACT-WITHDRAWAL: a declared contract its own analysed
   77: // body refutes is withdrawn from every caller, in its unit and, through the
   78: // audit's refutation table, in every other unit.
   79: #[path = "legacy_refuted_contract.rs"]
   80: pub mod refuted_contract;
   81: 
   82: // Card W45-SR5-SOUNDNESS: the reserved rewrite prefixes, escaped where the
   83: // source spells them, so a held or withdrawn call is only ever tool-minted.
   84: #[path = "legacy_reserved_spelling.rs"]
   85: pub mod reserved_spelling;
```

<a id="q46"></a>

## Q46 — `crates/qcore-core/src/concurrency.rs:1–85`

File SHA-256: `2d90e495a9b494d3048f01e4bbfc5256f998c346ec273e9e28868fb832165595`

```rust
    1: //! Cycle 14 restricted concurrency and resource-race checker.
    2: //!
    3: //! A program is an explicit event trace.  Vector clocks capture only
    4: //! sequenced-before, spawn/join, and mutex release/acquire edges; input order
    5: //! alone never orders different tasks.  This makes the checker useful as a
    6: //! bounded model without pretending to cover general C11 executions.
    7: //!
    8: //! Initial permissions/custody, guard-to-footprint authority and synchronized
    9: //! handoffs are trusted model inputs, not reconstructed source evidence. Every
   10: //! accepted certificate names that profile and its premises. Each resource ID
   11: //! denotes one unique lifetime; using a fresh ID is required for a new generation.
   12: //! Join is the explicit completion/collection event in this reduced model: it
   13: //! terminates the child without implicitly returning its permissions or custody.
   14: 
   15: use std::collections::{BTreeMap, BTreeSet};
   16: use std::fmt;
   17: 
   18: pub type TaskId = u32;
   19: /// A unique lifetime identity, never a reusable address or descriptor.
   20: pub type SharedResourceId = u32;
   21: pub type MutexId = u32;
   22: pub type LockRank = u32;
   23: pub const FULL_ACCESS: u16 = 1024;
   24: 
   25: #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
   26: pub struct AccessFootprint {
   27:     pub resource: SharedResourceId,
   28:     pub start: u32,
   29:     pub end: u32,
   30: }
   31: 
   32: impl AccessFootprint {
   33:     pub const fn new(resource: SharedResourceId, start: u32, end: u32) -> Self {
   34:         Self {
   35:             resource,
   36:             start,
   37:             end,
   38:         }
   39:     }
   40: 
   41:     pub const fn is_valid(self) -> bool {
   42:         self.start < self.end
   43:     }
   44: 
   45:     pub const fn overlaps(self, other: Self) -> bool {
   46:         self.resource == other.resource && self.start < other.end && other.start < self.end
   47:     }
   48: 
   49:     pub const fn contains(self, other: Self) -> bool {
   50:         self.resource == other.resource && self.start <= other.start && other.end <= self.end
   51:     }
   52: }
   53: 
   54: #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   55: pub enum AccessKind {
   56:     Read,
   57:     Write,
   58:     AtomicRead,
   59:     AtomicWrite,
   60: }
   61: 
   62: impl AccessKind {
   63:     const fn writes(self) -> bool {
   64:         matches!(self, Self::Write | Self::AtomicWrite)
   65:     }
   66: 
   67:     const fn atomic(self) -> bool {
   68:         matches!(self, Self::AtomicRead | Self::AtomicWrite)
   69:     }
   70: }
   71: 
   72: #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   73: pub struct AccessPermission {
   74:     pub task: TaskId,
   75:     pub footprint: AccessFootprint,
   76:     pub share: u16,
   77: }
   78: 
   79: #[derive(Clone, Debug, PartialEq, Eq)]
   80: pub enum ConcurrentEvent {
   81:     Spawn {
   82:         parent: TaskId,
   83:         child: TaskId,
   84:     },
   85:     Join {
```

<a id="q47"></a>

## Q47 — `crates/qcore-c-gateway/tests/condition_call_desugar.rs:280–330`

File SHA-256: `92c38f932db2baa7ebb2bf46d64b0a1b2e4f1eb0ee770c2a65b24691efc00c45`

```rust
  280:         }",
  281:     );
  282:     assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
  283:     assert_eq!(run.leaks(), 1);
  284: }
  285: 
  286: #[test]
  287: fn an_owned_result_assigned_in_a_condition_keeps_todays_exact_lowering() {
  288:     if !clang_available() {
  289:         return;
  290:     }
  291:     let run =
  292:         analyze("void subject(void) { void *p; if ((p = q_alloc()) == 0) { return; } q_free(p); }");
  293:     assert!(run.refused(BRANCH), "{run:?}");
  294:     assert_ne!(run.status, AnalysisStatus::Safe);
  295:     // The node is the unchanged refusal: no call was lifted out of it.
  296:     assert_eq!(run.calls("q_alloc"), 0);
  297:     assert!(run.cfg.nodes.iter().any(|node| matches!(
  298:         &node.instruction,
  299:         LegacyInstruction::Unknown { description, .. } if description == BRANCH
  300:     )));
  301:     // A discarded owned result stays refused too.
  302:     let run = analyze(
  303:         "void subject(void) { void *p = q_alloc(); if (r_wrap(p) != 0 && p) { } q_free(p); }",
  304:     );
  305:     assert!(run.refused(BRANCH), "{run:?}");
  306:     let run = analyze(
  307:         "void subject(int flag) { void *p = q_alloc(); if (flag && r_wrap(p)) { } q_free(p); }",
  308:     );
  309:     assert!(run.refused(BRANCH), "{run:?}");
  310: }
  311: 
  312: #[test]
  313: fn an_uncontracted_call_keeps_the_refusal() {
  314:     if !clang_available() {
  315:         return;
  316:     }
  317:     let run = analyze(
  318:         "void subject(int flag) { void *p = q_alloc(); if (flag && opaque()) { } q_free(p); }",
  319:     );
  320:     assert!(run.refused(BRANCH), "{run:?}");
  321:     assert_eq!(run.status, AnalysisStatus::Unknown);
  322: }
  323: 
  324: #[test]
  325: fn two_effectful_calls_of_one_operand_are_unsequenced_and_refused() {
  326:     if !clang_available() {
  327:         return;
  328:     }
  329:     let run = analyze(
  330:         "void subject(void) { void *p = q_alloc(); if (r_release_code(p) == r_borrow_code(p)) { } }",
```
