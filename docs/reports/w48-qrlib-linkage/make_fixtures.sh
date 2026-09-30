#!/bin/sh
# Card W48-QRLIB-LINKAGE: build the checked-in reader fixtures under
# crates/qcore-c-gateway/tests/fixtures/linkage/ with the host's Clang and
# binutils/LLVM tools, so every platform's reader is unit-tested from bytes on
# any host. Run from the repository root:
#
#   sh docs/reports/w48-qrlib-linkage/make_fixtures.sh
#
# Tools: clang (x86_64-linux-gnu, x86_64-apple-macos11 and
# x86_64-pc-windows-msvc targets, compiling only), ar, objcopy, strip,
# llvm-libtool-darwin, llvm-lib, llvm-dlltool. Every output is deterministic
# (no timestamps: `ar` D mode, `llvm-lib`/`llvm-libtool-darwin` defaults). The
# sources are written here, beside the commands, so the fixture is reviewable.
set -eu
out=crates/qcore-c-gateway/tests/fixtures/linkage
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$out/elf" "$out/macho" "$out/coff"

cat > "$work/qr_ok.c" <<'EOF'
int qr_add(int a, int b) { return a + b; }
__attribute__((weak)) int qr_weak(void) { return 1; }
static int qr_local_helper(void) { return 7; }
int qr_uses_helper(void) { return qr_local_helper(); }
extern int qr_undefined_elsewhere(void);
int qr_calls(void) { return qr_undefined_elsewhere(); }
int qr_localized(void) { return 3; }
int qr_stripped(void) { return 4; }
EOF
cat > "$work/qr_dup_a.c" <<'EOF'
int qr_dup(void) { return 1; }
EOF
cat > "$work/qr_dup_b.c" <<'EOF'
int qr_dup(void) { return 2; }
EOF

# ELF static archive: a local-only symbol (objcopy --localize-symbol), a
# stripped symbol (--strip-symbol), a weak definition, an undefined reference,
# and a duplicate definition across two members. The GNU index lists both.
for f in qr_ok qr_dup_a qr_dup_b; do
  clang --target=x86_64-linux-gnu -O1 -fno-asynchronous-unwind-tables -c "$work/$f.c" -o "$work/$f.o"
done
objcopy --localize-symbol=qr_localized --strip-symbol=qr_stripped "$work/qr_ok.o"
rm -f "$out/elf/libqr_demo.a" "$out/elf/libqr_noindex.a"
(cd "$work" && ar rcD libqr_demo.a qr_ok.o qr_dup_a.o qr_dup_b.o && ar rcSD libqr_noindex.a qr_ok.o)
cp "$work/libqr_demo.a" "$work/libqr_noindex.a" "$out/elf/"

# ELF shared object: a default-versioned symbol, a symbol with only a hidden
# version, a weak export, a hidden-visibility definition, an unversioned
# export is impossible under a version script, so none; DT_SONAME set.
cat > "$work/qr_versioned.c" <<'EOF'
int qr_v2(void) { return 2; }
int qr_old_v1(void) { return 10; }
int qr_old_v2(void) { return 20; }
int qr_legacy_v1(void) { return 30; }
__attribute__((weak)) int qr_soft(void) { return 5; }
__attribute__((visibility("hidden"))) int qr_hidden(void) { return 6; }
int qr_calls_hidden(void) { return qr_hidden(); }
__asm__(".symver qr_old_v1,qr_old@VERS_1");
__asm__(".symver qr_old_v2,qr_old@@VERS_2");
__asm__(".symver qr_legacy_v1,qr_legacy@VERS_1");
EOF
cat > "$work/qr_versioned.map" <<'EOF'
VERS_1 { global: qr_soft; qr_calls_hidden; qr_old; qr_legacy; local: *; };
VERS_2 { global: qr_v2; } VERS_1;
EOF
clang --target=x86_64-linux-gnu -O1 -fPIC -shared -nostdlib -fno-asynchronous-unwind-tables \
  -Wl,--version-script="$work/qr_versioned.map" -Wl,-soname,libqr_versioned.so.1 -Wl,--hash-style=gnu \
  -Wl,--build-id=none "$work/qr_versioned.c" -o "$work/libqr_versioned.so"
strip --strip-all "$work/libqr_versioned.so"
cp "$work/libqr_versioned.so" "$out/elf/"

# Mach-O static archive written by llvm-libtool-darwin, so it carries the
# ranlib `__.SYMDEF` index: a weak definition (N_WEAK_DEF), a private extern
# (N_PEXT: hidden visibility), a local static, an undefined reference.
cat > "$work/qr_mac.c" <<'EOF'
int qr_add(int a, int b) { return a + b; }
__attribute__((weak)) int qr_weak(void) { return 1; }
__attribute__((visibility("hidden"))) int qr_private(void) { return 2; }
static int qr_local_helper(void) { return 7; }
int qr_uses_helper(void) { return qr_local_helper() + qr_private(); }
extern int qr_undefined_elsewhere(void);
int qr_calls(void) { return qr_undefined_elsewhere(); }
EOF
clang --target=x86_64-apple-macos11 -O1 -fno-asynchronous-unwind-tables -c "$work/qr_mac.c" -o "$work/qr_mac.o"
rm -f "$out/macho/libqr_demo.a"
llvm-libtool-darwin -static -o "$out/macho/libqr_demo.a" "$work/qr_mac.o"

# COFF static library written by llvm-lib: first and second linker members.
cat > "$work/qr_win.c" <<'EOF'
int qr_add(int a, int b) { return a + b; }
static int qr_local_helper(void) { return 7; }
int qr_uses_helper(void) { return qr_local_helper(); }
extern int qr_undefined_elsewhere(void);
int qr_calls(void) { return qr_undefined_elsewhere(); }
EOF
clang --target=x86_64-pc-windows-msvc -O1 -c "$work/qr_win.c" -o "$work/qr_win.obj"
rm -f "$out/coff/qr_demo.lib"
# Run from the work directory: llvm-lib names a member by the path it was given.
(cd "$work" && llvm-lib /out:qr_demo.lib qr_win.obj)
cp "$work/qr_demo.lib" "$out/coff/"

# COFF import library written by llvm-dlltool: a by-name code import, an
# ordinal-only import, a data import, and two by-name imports the unit tests'
# synthesized `qr_runtime.dll` exports only as a forwarder (`qr_forwarded`) or
# not at all (`qr_gone`).
cat > "$work/qr_runtime.def" <<'EOF'
LIBRARY qr_runtime.dll
EXPORTS
  qr_by_name
  qr_by_ordinal @7 NONAME
  qr_data DATA
  qr_forwarded
  qr_gone
EOF
rm -f "$out/coff/qr_import.lib"
llvm-dlltool -m i386:x86-64 -d "$work/qr_runtime.def" -l "$out/coff/qr_import.lib"

printf '* binary\n' > "$out/.gitattributes"
sha256sum "$out"/elf/* "$out"/macho/* "$out"/coff/*
