# QCore syntax highlighting for Emacs

`qcore-mode.el` is a standalone major mode for Emacs 27.1 or later. It selects
QCore mode for `.qcore` and `.qc` files, uses standard theme faces, and supports
Emacs comment commands (`M-;`, `comment-region`, `uncomment-region`).

Add this to your Emacs initialization file, adjusting the checkout path:

```elisp
(add-to-list 'load-path "/path/to/qcore-c/editors/emacs")
(require 'qcore-mode)
```

Alternatively, load `qcore-mode.el` with `M-x load-file`, then use
`M-x qcore-mode` in an already open buffer. No external packages are required.
Use that same sequence after updating an already loaded version; `require`
alone does not reload it or refresh existing buffers.

## Implemented syntax coverage

The mode follows the current
[consumer grammars](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md),
[lexer](../../crates/qcore-frontend/src/lexer.rs),
[structural frontend](../../crates/qcore-frontend/src/frontend.rs), and
[dimension/axis parser](../../crates/qcore-frontend/src/parser.rs).

| Surface | Highlighting |
| --- | --- |
| Lexical syntax | `//` comments, non-nesting `/* ... */` comments, escaped strings, complete character literals, decimal integers and floats |
| Declarations | Modules, functions and function names, types, dimensions, axes, resource declarations, errors, verifier scopes |
| Executable source | Bindings, returns, acquisition/check builtins, drop/move, task spawn/join, captures, Result types, owned Buffer operations |
| Structural source | Control flow, error regions, effects, ownership/borrow and other structural types, lifetime names such as `'scope` |
| Quantitative syntax | Usage grades, credit/live/cost axes, closed indices, `where`, `min`, `max`, `monus` |
| Contracts | Clause introducers, exit selectors, Boolean operators, exact resource-loader v1 predicates and nullability modes |

Inside resource type bodies, `repr`, `state`, `footprint`, `destructor`, `copy`
and `move` are member keywords. Destructor targets use the function-name face,
state names the type face, `noexcept` the keyword face, and `forbidden`/`allowed`
the constant face. These additional rules use resource-body context so ordinary
parameters and locals named `repr`, `destructor` or `allowed` keep their binding
faces. Comments between member tokens are supported.

The lexer treats an apostrophe as a character delimiter only for one character
or one escaped character followed by a closing apostrophe. The mode does the
same, so `'x'` is a character while `'scope` is a lifetime. Editing between these
forms updates the syntax properties. Comment/string contents take precedence
over keyword highlighting. Identifiers include underscores and matching is
case-sensitive. Angle brackets remain punctuation because they also express
comparisons.

Highlighting is lexical assistance, not a conformance or verification result.
Some structurally recognized forms have no executable lowering. Arbitrary
contract expressions can be retained by the structural parser without being
interpreted; the mode does not mark arbitrary `resource_*` names as builtins.
Names in declarations and before colons are highlighted by their surrounding
syntax; this does not resolve user-defined types or names across a program.
The mode does not implement semantic diagnostics, completion, compiler commands,
or language-specific indentation. Configuration JSON and unrestricted C files
continue to use their respective modes.

## Validation

From the repository root:

```sh
emacs -Q --batch -L editors/emacs -l qcore-mode-tests.el -f ert-run-tests-batch-and-exit
```

The regressions check face precedence, character/lifetime edits, escaped
newlines, non-nesting comments, file selection, comment commands, and loading
the existing contract, example and CT fixture corpus. The intrusive-list
resource declaration has explicit face checks, with separate controls for
resource members, ordinary bindings, nested fields and comments.
