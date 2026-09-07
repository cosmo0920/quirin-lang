;;; qcore-mode.el --- Highlight QCore source and contracts -*- lexical-binding: t; -*-

;; Version: 0.1.0
;; Package-Requires: ((emacs "27.1"))
;; Keywords: languages

;;; Commentary:

;; A standalone major mode for the implemented QCore revision-0.3 lexical,
;; structural and contract syntax.  Highlighting does not imply that a form
;; can be executed or that a contract has been verified.  See this directory's
;; README.md for installation, scope and the implementation sources.
;; No compiler, tree-sitter grammar or external package is required.

;;; Code:

(require 'prog-mode)
(require 'font-lock)
(require 'syntax)

(defconst qcore-mode--keywords
  '("module" "executable" "library" "contract" "dimension" "quant" "axis"
    "type" "resource" "error" "const" "extern" "fn" "concurrency" "checked"
    "verify" "scope" "plugin" "version" "profile" "capabilities"
    "effects" "requires" "ensures" "normal" "any" "use" "as" "idx"
    "let" "drop" "move" "discharge" "guard" "return" "raise" "break"
    "continue" "defer" "if" "else" "match" "while" "invariant" "bound"
    "region" "catch" "open" "unsafe" "thread" "with" "lock" "spawn"
    "join" "copy" "read" "write" "share" "mut" "footprint" "range"
    "where" "and" "or" "not")
  "Words recognized by the structural source and predicate consumers.
Some structural forms have no executable lowering yet.")

(defconst qcore-mode--types
  '("unit" "bool" "i8" "i16" "i32" "i64" "isize" "u8" "u16" "u32"
    "u64" "usize" "f32" "f64" "array" "slice" "ptr" "own" "borrow"
    "result" "never" "task" "mutex" "Idx" "Ordered" "Count" "Buffer"
    "Result" "AllocError" "SpawnError")
  "Primitive and structural type vocabulary, plus executable task types.")

(defconst qcore-mode--builtins
  '("credit" "live" "cost" "total" "peak" "access" "min" "max" "monus"
    "alloc" "throws" "may_block" "qcore_acquire" "qcore_check"
    "qcore_buffer_new" "qcore_buffer_read" "resource_contract_v1"
    "resource_borrowed" "resource_owned" "resource_consumed"
    "resource_released" "resource_out" "resource_stored" "resource_removed"
    "resource_return_none" "resource_return_owned" "resource_return_reallocated")
  "Implemented quantitative, task and resource-contract vocabulary.
Arbitrary retained contract predicates are not treated as builtins.")

(defconst qcore-mode--constants
  '("true" "false" "zero" "one" "affine" "relevant" "many" "inf" "FULL"
    "nullable" "nonnull")
  "Boolean literals, grades, access constant and resource nullability modes.")

(defvar qcore-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?_ "w" table)
    ;; QCore block comments do not nest.
    (modify-syntax-entry ?/ ". 124b" table)
    (modify-syntax-entry ?* ". 23" table)
    (modify-syntax-entry ?\n "> b" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    ;; Only complete character literals receive string syntax below.
    (modify-syntax-entry ?' "." table)
    ;; Angle brackets also express comparisons; do not make them paired.
    (dolist (char '(?< ?> ?+ ?- ?= ?! ?& ?% ?? ?: ?. ?, ?\;))
      (modify-syntax-entry char "." table))
    table)
  "Syntax table for QCore source, shared by executable and contract modules.")

(defconst qcore-mode--character-regexp
  "'\\(?:[^\\\\'\n\r]\\|\\\\\\(?:.\\|\n\\)\\)'"
  "A character or one escaped character, matching Lexer::starts_char_literal.")

(defun qcore-mode--syntax-propertize (start end)
  "Mark complete character literals between START and END as strings.
Leave lifetime apostrophes as punctuation, including incomplete literals."
  (goto-char start)
  (while (re-search-forward qcore-mode--character-regexp end t)
    (let ((begin (match-beginning 0))
          (finish (match-end 0)))
      (unless (nth 8 (save-excursion (syntax-ppss begin)))
        (put-text-property begin (1+ begin) 'syntax-table (string-to-syntax "\""))
        (put-text-property (1- finish) finish 'syntax-table (string-to-syntax "\""))
        (put-text-property begin finish 'syntax-multiline t)))))

(defconst qcore-mode-font-lock-keywords
  (let ((identifier "[A-Za-z_][A-Za-z_0-9]*"))
    `((,(concat "'" identifier "\\_>") . font-lock-variable-name-face)
      (,(regexp-opt qcore-mode--keywords 'symbols) . font-lock-keyword-face)
      (,(concat "\\_<fn\\_>[ \t\n]+\\(" identifier "\\)")
       1 font-lock-function-name-face)
      (,(concat "\\_<\\(?:type\\|dimension\\|error\\)\\_>[ \t\n]+\\("
                identifier "\\)\\_>")
       1 font-lock-type-face)
      (,(concat "\\_<axis\\_>[ \t\n]+\\(" identifier "\\)")
       1 font-lock-variable-name-face)
      (,(regexp-opt qcore-mode--types 'symbols) . font-lock-type-face)
      (,(regexp-opt qcore-mode--builtins 'symbols) . font-lock-builtin-face)
      (,(regexp-opt qcore-mode--constants 'symbols) . font-lock-constant-face)
      ;; Names in bindings/parameters/fields; this is lexical, not name resolution.
      (,(concat "\\_<let\\_>[ \t\n]+\\(?:idx[ \t\n]+\\)?\\("
                identifier "\\)")
       1 font-lock-variable-name-face)
      (,(concat "\\_<\\(" identifier "\\)\\_>[ \t]*:[^:=]")
       1 font-lock-variable-name-face)
      ("\\_<[0-9]+\\(?:\\.[0-9]+\\)?\\_>" . font-lock-constant-face)))
  "Font Lock rules for QCore's currently implemented consumers.")

;;;###autoload
(define-derived-mode qcore-mode prog-mode "QCore"
  "Major mode for highlighting QCore source and contract files.
This mode provides lexical highlighting and comment commands, not validation
or language-specific indentation.  See `qcore-mode-font-lock-keywords'."
  :syntax-table qcore-mode-syntax-table
  (setq-local font-lock-defaults '(qcore-mode-font-lock-keywords))
  (setq-local syntax-propertize-function #'qcore-mode--syntax-propertize)
  (add-hook 'syntax-propertize-extend-region-functions
            #'syntax-propertize-wholelines nil t)
  (add-hook 'syntax-propertize-extend-region-functions
            #'syntax-propertize-multiline nil t)
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "\\(?://+\\|/\\*+\\)[ \t]*"))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.\\(?:qcore\\|qc\\)\\'" . qcore-mode))

(provide 'qcore-mode)
;;; qcore-mode.el ends here
