/-! Finite selected loader invocation model. Toolchain, source extraction,
explicit stub environment and C/model correspondence are prerequisites. This
model neither discharges old universal plugin premises nor changes verdicts. -/
namespace QCore.PluginSourceWitnesses

inductive Case where
  | rk_duplicate
  | rk_open_fail
  | rk_conf_init_missing
  | rk_conf_init_error
  | rk_load_success
  | rk_load_destroy
  | flb_c_open_fail
  | flb_c_invalid_name
  | flb_c_missing_structure
  | flb_c_input_success
  | flb_c_processor_success
  | flb_c_filter_success
  | flb_c_output_success
  | flb_c_context_alloc_fail
  | go_pre_optional_absent
  | go_pre_failed
  | go_register_missing
  | go_init_missing
  | go_optional_hooks_absent
  | go_create_ignores_register_failure
  deriving DecidableEq

def expected : Case → List String
  | .rk_duplicate => ["begin", "duplicate_test", "return:0", "end"]
  | .rk_open_fail => ["begin", "duplicate_test", "open", "return:error", "end"]
  | .rk_conf_init_missing => ["begin", "duplicate_test", "open", "symbol:conf_init", "close", "return:error", "end"]
  | .rk_conf_init_error => ["begin", "duplicate_test", "open", "symbol:conf_init", "conf_init", "close", "return:error", "end"]
  | .rk_load_success => ["begin", "duplicate_test", "open", "symbol:conf_init", "conf_init", "allocate", "allocate", "retain", "return:0", "end"]
  | .rk_load_destroy => ["begin", "duplicate_test", "open", "symbol:conf_init", "conf_init", "allocate", "allocate", "retain", "close", "free", "free", "return:0", "end"]
  | .flb_c_open_fail => ["begin", "open", "return:error", "end"]
  | .flb_c_invalid_name => ["begin", "open", "close", "return:error", "end"]
  | .flb_c_missing_structure => ["begin", "open", "allocate", "copy", "copy", "symbol:in_missing_plugin", "free", "close", "return:error", "end"]
  | .flb_c_input_success => ["begin", "open", "allocate", "copy", "copy", "symbol:in_probe_plugin", "allocate", "copy", "link", "free", "allocate", "retain", "link", "return:0", "end"]
  | .flb_c_processor_success => ["begin", "open", "allocate", "copy", "copy", "symbol:processor_probe_plugin", "allocate", "copy", "link", "free", "allocate", "retain", "link", "return:0", "end"]
  | .flb_c_filter_success => ["begin", "open", "allocate", "copy", "copy", "symbol:filter_probe_plugin", "allocate", "copy", "link", "free", "allocate", "retain", "link", "return:0", "end"]
  | .flb_c_output_success => ["begin", "open", "allocate", "copy", "copy", "symbol:out_probe_plugin", "allocate", "copy", "link", "free", "allocate", "retain", "link", "return:0", "end"]
  | .flb_c_context_alloc_fail => ["begin", "open", "allocate", "copy", "copy", "symbol:in_probe_plugin", "allocate", "copy", "link", "free", "allocate", "close", "return:error", "end"]
  | .go_pre_optional_absent => ["begin", "open", "symbol:FLBPluginPreRegister", "symbol:FLBPluginRegister", "register", "allocate", "symbol:FLBPluginInit", "symbol:FLBPluginFlush", "symbol:FLBPluginFlushCtx", "symbol:FLBPluginExit", "symbol:FLBPluginExitCtx", "allocate", "link", "return:0", "init:present", "optional:absent", "close", "end"]
  | .go_pre_failed => ["begin", "open", "symbol:FLBPluginPreRegister", "pre_register", "return:error", "close", "end"]
  | .go_register_missing => ["begin", "open", "symbol:FLBPluginPreRegister", "symbol:FLBPluginRegister", "return:error", "close", "end"]
  | .go_init_missing => ["begin", "open", "allocate", "symbol:FLBPluginInit", "free", "return:error", "close", "end"]
  | .go_optional_hooks_absent => ["begin", "open", "allocate", "symbol:FLBPluginInit", "symbol:FLBPluginFlush", "symbol:FLBPluginFlushCtx", "symbol:FLBPluginExit", "symbol:FLBPluginExitCtx", "allocate", "return:0", "init:present", "optional:absent", "close", "end"]
  | .go_create_ignores_register_failure => ["begin", "open", "allocate", "allocate", "allocate", "link", "symbol:FLBPluginPreRegister", "symbol:FLBPluginRegister", "return:nonnull", "free", "close", "free", "end"]

def complete (traces : Case → List String) : Prop :=
  ∀ selected, traces selected = expected selected

def DuplicateNoLoaderCalls (trace : List String) : Prop :=
  trace = ["begin", "duplicate_test", "return:0", "end"]

structure Context where
  source artifact build property bounds environment : String
  deriving DecidableEq

/-- Independent source/native replay, not presentation agreement alone. -/
def admitted (presented actual : Case → List String)
    (presentedContext actualContext : Context) (sourceProjection typedGuards nativeArtifacts : Prop) : Prop :=
  sourceProjection ∧ typedGuards ∧ nativeArtifacts ∧ complete actual ∧
  (∀ selected, presented selected = actual selected) ∧ presentedContext = actualContext

theorem duplicate_finite_fact (traces : Case → List String)
    (checked : complete traces) : DuplicateNoLoaderCalls (traces .rk_duplicate) := by
  exact checked .rk_duplicate

theorem duplicate_has_no_open (traces : Case → List String)
    (checked : complete traces) : "open" ∉ traces .rk_duplicate := by
  rw [checked .rk_duplicate]
  simp [expected]

theorem changed_case_refuses (presented actual : Case → List String)
    (pc ac : Context) (source typed native : Prop) (selected : Case)
    (changed : presented selected ≠ actual selected) :
    ¬ admitted presented actual pc ac source typed native := by
  intro checked
  exact changed (checked.2.2.2.2.1 selected)

theorem changed_context_refuses (presented actual : Case → List String)
    (pc ac : Context) (source typed native : Prop) (changed : pc ≠ ac) :
    ¬ admitted presented actual pc ac source typed native := by
  intro checked
  exact changed checked.2.2.2.2.2

theorem missing_source_refuses (presented actual : Case → List String)
    (pc ac : Context) (source typed native : Prop) (missing : ¬ source) :
    ¬ admitted presented actual pc ac source typed native := by
  intro checked
  exact missing checked.1

theorem admitted_preserves_exact_context (presented actual : Case → List String)
    (pc ac : Context) (source typed native : Prop)
    (checked : admitted presented actual pc ac source typed native) : pc = ac :=
  checked.2.2.2.2.2

/-- This finite observation is deliberately weaker than registration success. -/
theorem creator_return_is_not_registration_evidence :
  "return:nonnull" ∈ expected .go_create_ignores_register_failure ∧
  "register" ∉ expected .go_create_ignores_register_failure := by
  simp [expected]

/-- The later allocation failure retains a prior selected link event. -/
theorem c_context_failure_does_not_erase_prior_link :
  "link" ∈ expected .flb_c_context_alloc_fail ∧
  "return:error" ∈ expected .flb_c_context_alloc_fail := by
  simp [expected]

/-- No original-premise or verdict argument is used by finite admission. -/
theorem old_truth_remains_independent (oldTruth : Prop)
    (presented actual : Case → List String) (pc ac : Context)
    (source typed native : Prop) :
    (admitted presented actual pc ac source typed native ∧ oldTruth) ↔
    (oldTruth ∧ admitted presented actual pc ac source typed native) := by
  exact and_comm

end QCore.PluginSourceWitnesses
