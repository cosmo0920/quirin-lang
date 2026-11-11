;;; qcore-mode-tests.el --- QCore highlighting regressions -*- lexical-binding: t; -*-
;; SPDX-License-Identifier: Apache-2.0
;;
;; Licensed under the Apache License, Version 2.0 (the "License");
;; you may not use this file except in compliance with the License.
;; You may obtain a copy of the License at
;;
;;     http://www.apache.org/licenses/LICENSE-2.0
;;
;; Unless required by applicable law or agreed to in writing, software
;; distributed under the License is distributed on an "AS IS" BASIS,
;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;; See the License for the specific language governing permissions and
;; limitations under the License.


;;; Code:

(require 'ert)
(require 'qcore-mode)

(defconst qcore-mode-test--root
  (expand-file-name "../.." (file-name-directory (or load-file-name buffer-file-name))))

(defun qcore-mode-test--face (text)
  "Return the face at the start of the first occurrence of TEXT."
  (goto-char (point-min))
  (let ((case-fold-search nil))
    (search-forward text))
  (get-text-property (- (point) (length text)) 'face))

(ert-deftest qcore-mode-file-associations ()
  (dolist (name '("program.qr" "contract.qv" "legacy.qcore"))
    (with-temp-buffer
      (setq buffer-file-name name)
      (set-auto-mode)
      (should (eq major-mode 'qcore-mode)))))

(ert-deftest qcore-mode-retired-qc-extension ()
  "`.qc' is retired: it must not select `qcore-mode'."
  (dolist (name '("cycle12.qc" "program.qc"))
    (with-temp-buffer
      (setq buffer-file-name name)
      (set-auto-mode)
      (should-not (eq major-mode 'qcore-mode)))))

(ert-deftest qcore-mode-source-and-contract-faces ()
  (with-temp-buffer
    (insert "module contract;\ndimension Bytes : Idx + Ordered;\n"
            "quant axis chunks : credit<Count<Chunk>>;\n"
            "extern fn keep(q: quant<n where n := 5> use one) -> unit\n"
            "contract { requires resource_contract_v1;\n"
            "ensures normal resource_owned(q, nonnull); };\n"
            "fn main() -> i32 effects { alloc, throws<AllocError>, spawn } {\n"
            "let buffer = qcore_buffer_new(42)?; thread scope s {\n"
            "let h = spawn s worker(move buffer)?; let y = join move h?;\n"
            "drop y; return 0; }}\n"
            "let alias: borrow<'scope, const, Buffer>;\n"
            "let ratio = 12.34; let true_value = 1;\n"
            "requires resource_future(q);\n")
    (qcore-mode)
    (font-lock-ensure)
    (dolist (case '(("module" . font-lock-keyword-face)
                    ("Bytes" . font-lock-type-face)
                    ("chunks" . font-lock-variable-name-face)
                    ("keep" . font-lock-function-name-face)
                    ("main" . font-lock-function-name-face)
                    ("where" . font-lock-keyword-face)
                    ("one" . font-lock-constant-face)
                    ("resource_owned" . font-lock-builtin-face)
                    ("nonnull" . font-lock-constant-face)
                    ("qcore_buffer_new" . font-lock-builtin-face)
                    ("Buffer" . font-lock-type-face)
                    ("'scope" . font-lock-variable-name-face)
                    ("12.34" . font-lock-constant-face)
                    ("true_value" . font-lock-variable-name-face)))
      (should (eq (qcore-mode-test--face (car case)) (cdr case))))
    (should-not (qcore-mode-test--face "resource_future"))))

(ert-deftest qcore-mode-comments-strings-and-lifetimes ()
  (with-temp-buffer
    (insert "// fn fake() 'a'\n/* outer /* inner */ fn real() {}\n"
            "let text = \"// 'a' /* fn quoted \\\" end\";\n"
            "let ch = 'x'; let quote = '\"'; let slash = '/';\n"
            "let escaped = '\\n'; let apostrophe = '\\''; let unicode = '界';\n"
            "let a: borrow<'scope, const, Buffer>; let b: task<'s, i32>;\n"
            "return 0;\n")
    (qcore-mode)
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "fake") 'font-lock-comment-face))
    (should (eq (qcore-mode-test--face "real") 'font-lock-function-name-face))
    (dolist (text '("quoted" "'x'" "'\"'" "'/'" "'\\n'" "'\\''" "'界'"))
      (should (eq (qcore-mode-test--face text) 'font-lock-string-face)))
    (dolist (text '("'scope" "'s,"))
      (ert-info ((format "Lifetime %s" text))
        (should (eq (qcore-mode-test--face text) 'font-lock-variable-name-face))))
    (should (eq (qcore-mode-test--face "return") 'font-lock-keyword-face))
    (should-not (nth 8 (syntax-ppss (point-max))))))

(ert-deftest qcore-mode-edit-character-into-lifetime-and-back ()
  (with-temp-buffer
    (insert "'a' return 0;\n")
    (qcore-mode)
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "'a'") 'font-lock-string-face))
    (goto-char 3)
    (delete-char 1)
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "'a") 'font-lock-variable-name-face))
    (should (eq (qcore-mode-test--face "return") 'font-lock-keyword-face))
    (goto-char 3)
    (insert "'")
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "'a'") 'font-lock-string-face))
    (should (eq (qcore-mode-test--face "return") 'font-lock-keyword-face))))

(ert-deftest qcore-mode-escaped-newline-character ()
  (with-temp-buffer
    ;; The current lexer admits one escaped newline as a character token.
    (insert "'\\\n' return 0;\n")
    (qcore-mode)
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "'\\\n'") 'font-lock-string-face))
    (should (eq (qcore-mode-test--face "return") 'font-lock-keyword-face))
    (goto-char 4)
    (delete-char 1)
    (font-lock-ensure)
    (should-not (nth 8 (syntax-ppss (point-max))))))

(ert-deftest qcore-mode-comment-command ()
  (with-temp-buffer
    (insert "return 0;\n")
    (qcore-mode)
    (comment-region (point-min) (point-max))
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "return") 'font-lock-comment-face))
    (uncomment-region (point-min) (point-max))
    (should (equal (buffer-string) "return 0;\n"))))

(ert-deftest qcore-mode-current-corpus-smoke ()
  (dolist (directory '("contract/qcore" "examples" "compiler/tests/fixtures/ct"))
    (dolist (file (directory-files-recursively
                   (expand-file-name directory qcore-mode-test--root)
                   "\\.\\(qv\\|qr\\|qcore\\)\\'"))
      (with-temp-buffer
        (insert-file-contents file)
        (qcore-mode)
        (font-lock-ensure)
        (should-not (nth 8 (syntax-ppss (point-max))))))))

(ert-deftest qcore-mode-intrusive-list-resource-members ()
  (with-temp-buffer
    (insert-file-contents
     (expand-file-name "providers/quirin-provider-cfl/contracts/cfl/intrusive_list.qcore" qcore-mode-test--root))
    (qcore-mode)
    (font-lock-ensure)
    (dolist (word '("repr" "footprint" "destructor" "copy" "move" "noexcept"))
      (should (eq (qcore-mode-test--face word) 'font-lock-keyword-face)))
    (should (eq (qcore-mode-test--face "cfl_list_entry_destroy")
                'font-lock-function-name-face))
    (dolist (word '("forbidden" "allowed"))
      (should (eq (qcore-mode-test--face word) 'font-lock-constant-face)))))

(ert-deftest qcore-mode-resource-context-and-trivia ()
  (with-temp-buffer
    (insert "resource /* comment */ type Item[T: Type] {\n"
            "state: Open; destructor /* field */ :\n"
            "  release_item /* function */ noexcept;\n"
            "footprint: { destructor: 1 }; copy: forbidden; move: allowed;\n"
            "}\nfn f(repr: i32, destructor: i32, state: i32) -> unit {\n"
            "let allowed = 1; let forbidden = 2; let noexcept = 3;\n"
            "let text = \"destructor: fake noexcept;\";\n"
            "// destructor: commented noexcept;\nreturn; }\n")
    (qcore-mode)
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "Open") 'font-lock-type-face))
    (should (eq (qcore-mode-test--face "release_item") 'font-lock-function-name-face))
    (should (eq (qcore-mode-test--face "noexcept;") 'font-lock-keyword-face))
    (dolist (text '("destructor: 1" "repr: i32" "destructor: i32" "state: i32"
                    "allowed =" "forbidden =" "noexcept ="))
      (should (eq (qcore-mode-test--face text) 'font-lock-variable-name-face)))
    (should (eq (qcore-mode-test--face "fake") 'font-lock-string-face))
    (should (eq (qcore-mode-test--face "commented") 'font-lock-comment-face))
    (goto-char (point-min))
    (search-forward "release_item")
    (replace-match "dispose_item" t t)
    (font-lock-ensure)
    (should (eq (qcore-mode-test--face "dispose_item") 'font-lock-function-name-face))))

;;; qcore-mode-tests.el ends here
