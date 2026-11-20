#!/bin/sh
# Attribution variant (not committed): the tip with the store's cleanup key
# spelled x509_store_st.cleanup_fn, so no run unit names a segment of it that
# wasm_loader.c names. Lock re-pinned in the side tree, then the opt-in run.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a53044d2541350255
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
BIN=$W/private/side-base/target/release/quirin-audit
V=$W/private/side-variant
L=$W/private/logs
SCRIPT=$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh
rm -rf "$V"; mkdir -p "$V"
cd "$W" || exit 2
git archive 8fffb83ae | tar -x -C "$V"
C=$V/providers/quirin-provider-openssl
sed -i 's/x509_store_st\.cleanup\b/x509_store_st.cleanup_fn/g' "$C/src/documented.rs" \
  "$C/contracts/openssl/x509_store_slots.qv" "$C/contracts/openssl/ssl_io.qv"
grep -c "cleanup_fn" "$C/src/documented.rs" "$C/contracts/openssl/x509_store_slots.qv" "$C/contracts/openssl/ssl_io.qv" > "$L/variant-rename.log"
cd "$V/providers" || exit 2
sh "$ENV" "$W/providers/target/debug/quirin-plugin" load-path --extends contract-load-path.json \
  --output user-crates-load-path.json quirin-provider-libcurl quirin-provider-openssl > "$L/variant-load-path.log" 2>&1
echo $? > "$L/variant-load-path.exit"
cd "$V" || exit 2
sh "$ENV" python3 providers/package.py checkers > "$L/variant-checkers-1.log" 2>&1; echo $? > "$L/variant-checkers-1.exit"
sh "$ENV" "$BIN" plugin views --load-path providers/user-crates-load-path.json --write > "$L/variant-views.log" 2>&1; echo $? > "$L/variant-views.exit"
sh "$ENV" "$BIN" pin --load-path providers/user-crates-load-path.json \
  --source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture \
  --crate quirin-provider-openssl > "$L/variant-pin.log" 2>&1; echo $? > "$L/variant-pin.exit"
sh "$ENV" python3 providers/contracts.py sync > "$L/variant-sync.log" 2>&1; echo $? > "$L/variant-sync.exit"
sh "$SCRIPT" "$V" "$W/private/opt-in/variant-1" "$BIN" "$W/private/fb-046accf6"
echo "checkers1 $(cat "$L/variant-checkers-1.exit") views $(cat "$L/variant-views.exit") pin $(cat "$L/variant-pin.exit") sync $(cat "$L/variant-sync.exit") run-checkers $(cat "$W/private/opt-in/variant-1/checkers.exit") run $(cat "$W/private/opt-in/variant-1/exit")"
