#!/usr/bin/env sh
# M-LINK.0 Unix spike (Linux / macOS).
#
# Usage: unix.sh <linux|macos>
#
# Mirrors scripts/windows.ps1: emit executable objects, then link them with the
# platform C driver (and ld.lld when available) with NO rustc/cargo in the link
# step, run every fixture, and record evidence under report/<os>/.
#
# The system library list is derived from the arguments rustc itself passes to
# the platform linker, so it reflects the toolchain's real requirements instead
# of guesswork. Rustc is used here only to discover dependencies; it is not part
# of the final link.
set -eu

OS_NAME="${1:?usage: unix.sh <linux|macos>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
OUT="$ROOT/out"
REPORT="$ROOT/report/$OS_NAME"
mkdir -p "$OUT" "$REPORT"

CC="${CC:-cc}"
ARCH="$(uname -m)"
LOG="$REPORT/spike.log"
: > "$LOG"
log() { echo "$*" | tee -a "$LOG"; }

fixtures="fx-core-only fx-async fx-stdlib fx-http"
need_stdlib() { [ "$1" = "fx-stdlib" ] || [ "$1" = "fx-http" ]; }
FAILED=0

# --- build reference runtime archives + spike tooling -------------------
if [ "${SKIP_BUILD:-0}" != "1" ]; then
  log "building reference staticlibs (cargo)..."
  cargo build -p vut-runtime -p vut-stdlib-native --manifest-path "$REPO/Cargo.toml" \
    > "$REPORT/cargo-build.log" 2>&1
  if [ ! -x "$ROOT/objgen/target/release/vut-objgen" ]; then
    cargo build --release --manifest-path "$ROOT/objgen/Cargo.toml" \
      > "$REPORT/objgen-build.log" 2>&1
  fi
fi

OBJGEN="$ROOT/objgen/target/release/vut-objgen"
CORE="$REPO/target/debug/libvut_runtime.a"
STDLIB="$REPO/target/debug/libvut_stdlib_native.a"
[ -f "$CORE" ] || { echo "missing $CORE" >&2; exit 1; }
[ -f "$STDLIB" ] || { echo "missing $STDLIB" >&2; exit 1; }
[ -x "$OBJGEN" ] || { echo "missing $OBJGEN" >&2; exit 1; }

# --- startup object -----------------------------------------------------
rustc --edition 2024 --crate-type=lib --emit=obj -C panic=abort -C opt-level=2 \
  -o "$OUT/vut-startup.o" "$ROOT/startup/vut-startup.rs"

# --- derive the platform system libraries rustc uses --------------------
capture_system_libs() {
  cat > "$OUT/linkwrap.sh" <<'WRAP'
#!/bin/sh
printf '%s\n' "$*" >> "$VUT_LINKARG_LOG"
exec "$VUT_REAL_LINK" "$@"
WRAP
  chmod +x "$OUT/linkwrap.sh"
  : > "$OUT/linkargs.txt"
  printf 'fn main() {}\n' > "$OUT/hello.rs"
  if ! VUT_LINKARG_LOG="$OUT/linkargs.txt" VUT_REAL_LINK="$CC" \
      rustc --edition 2021 -C linker="$OUT/linkwrap.sh" -o "$OUT/hello-rust" "$OUT/hello.rs" \
      > "$REPORT/capture.log" 2>&1; then
    log "warning: rustc link-arg capture failed; falling back to a baseline list"
    echo "-lpthread -ldl -lm -lrt -lutil -lgcc_s"
    return 0
  fi
  # Only whole tokens that begin with -l (and -framework <name> pairs) count;
  # substring matches inside paths like .../x86_64-unknown-linux-gnu/lib/... must
  # be ignored.
  awk '{
    for (i = 1; i <= NF; i++) {
      if ($i ~ /^-l./) { print $i }
      else if ($i == "-framework") { print "-framework " $(i + 1) }
    }
  }' "$OUT/linkargs.txt" | awk '!seen[$0]++' | tr '\n' ' '
}

SYSTEM_LIBS="$(capture_system_libs)"
log "arch: $ARCH"
log "system libs (from rustc): $SYSTEM_LIBS"

# Link-flag variants to try, most likely first. Cranelift currently emits
# non-position-independent objects, which glibc accepts with -no-pie and macOS
# x86_64 accepts with -Wl,-no_pie but macOS arm64 rejects (no_pie unsupported).
link_variants() {
  case "$OS_NAME" in
    linux) printf '%s\n' "-no-pie" "" ;;
    macos) printf '%s\n' "-Wl,-no_pie" "" ;;
    *) printf '%s\n' "" ;;
  esac
}

# The system-library capture uses a dependency-free hello world, so framework
# requirements that only the stdlib/HTTP stack pulls in (getrandom's Security
# framework on macOS) are not visible there and are added explicitly.
MAC_STDLIB_FRAMEWORKS=""
if [ "$OS_NAME" = "macos" ]; then
  MAC_STDLIB_FRAMEWORKS="-framework Security -framework CoreFoundation"
fi

run_fixture() {
  exe="$1"; tag="$2"
  set +e
  "$exe" > "$REPORT/$tag.stdout.txt" 2> "$REPORT/$tag.stderr.txt"
  code=$?
  set -e
  stdout="$(tr -d '\r' < "$REPORT/$tag.stdout.txt" | tr '\n' '|')"
  log "  run  exit=$code stdout='$stdout'"
}

link_with() {
  fx="$1"
  obj="$OUT/$fx.o"
  if need_stdlib "$fx"; then libs="$CORE $STDLIB"; fw="$MAC_STDLIB_FRAMEWORKS"; else libs="$CORE"; fw=""; fi
  for variant in $(link_variants); do
    label="$(echo "$variant" | tr -d ' ' | sed 's/^$/default/')"
    exe="$OUT/$fx-cc-$label"
    logfile="$REPORT/$fx-cc-$label.log"
    # shellcheck disable=SC2086
    if $CC $variant -o "$exe" "$OUT/vut-startup.o" "$obj" $libs $SYSTEM_LIBS $fw \
        > "$logfile" 2>&1; then
      log "  cc[$variant] link=ok -> $(basename "$exe")"
      run_fixture "$exe" "$fx-cc-$label"
      return 0
    fi
    errs="$(grep -cE 'undefined reference|text-relocation|error|not found|no_pie' "$logfile" || true)"
    log "  cc[$variant] link=FAIL errors=$errs"
    grep -E 'undefined reference|text-relocation|error|not found|no_pie' "$logfile" | head -n 6 | tee -a "$LOG"
  done
  FAILED=1
  log "  $fx FAILED to link with any variant"
}

log "=== unix spike on $OS_NAME ($ARCH, cc=$CC) ==="
for fx in $fixtures; do
  log "=== $fx ==="
  "$OBJGEN" "$ROOT/fixtures/$fx" "$OUT/$fx.o" > /dev/null
  link_with "$fx"
done

log "M-LINK.0 $OS_NAME spike complete."
exit "$FAILED"
