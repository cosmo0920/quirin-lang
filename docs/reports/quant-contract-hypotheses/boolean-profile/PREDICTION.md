# Bounded Boolean source condition migration — before implementation

Base: 6ac91b855 plus the explicit-entry graph integration. Extend the existing
bounded Boolean source-contract reader with exact typed entry-role definitions
and its existing integer equality/add/subtract postcondition fragment. Migrate
CMetrics reset and ChunkIO busy/lock policies (four functions / three files).
Keep every existing signature, modeled-domain, complete-path and ghost-variable
check. Unknown roles, unsupported exit selectors and unsupported expressions
refuse; normal conditions never become prerequisites.

Live/exclusive/Boolean-domain conditions remain explicit caller assumptions,
not newly derived C facts. Bind the full condition definitions into task
assumption identities as well as the existing contract source identity. No
condition interpretation/evidence is issued to generic hypothesis graphs and
no ordinary C summary is installed by this reader. Both legacy and typed source
spellings must preserve bounded results. Predict zero live corpus/system/slice
verdict moves; negative mutation cases must continue refusing.
