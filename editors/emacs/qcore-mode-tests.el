;;; qcore-mode-tests.el --- QCore highlighting regressions -*- lexical-binding: t; -*-

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
  (dolist (name '("program.qcore" "cycle12.qc"))
    (with-temp-buffer
      (setq buffer-file-name name)
      (set-auto-mode)
      (should (eq major-mode 'qcore-mode)))))

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
                   "\\.\\(?:qcore\\|qc\\)\\'"))
      (with-temp-buffer
        (insert-file-contents file)
        (qcore-mode)
        (font-lock-ensure)
        (should-not (nth 8 (syntax-ppss (point-max))))))))

;;; qcore-mode-tests.el ends here
