#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export PATH="$HOME/.cargo/bin:$PATH"
export RUSTC="$(rustup which --toolchain stable rustc)"

mdbook build

# Search Console's ownership file must reach the site root byte for byte.
VERIFICATION="googlefb1695ee9a07673e.html"
if [ "$(cat "book/$VERIFICATION" 2>/dev/null; printf x)" != "google-site-verification: ${VERIFICATION}x" ]; then
    echo "book/$VERIFICATION is missing or altered"
    exit 1
fi
echo "ownership file ok"

python3 - "$ROOT/docs" <<'PY'
import os, re, sys

docs = sys.argv[1]
for name in sorted(os.listdir(docs)):
    if not name.endswith(".md"):
        continue
    text = open(os.path.join(docs, name)).read()
    for m in re.finditer(r"\]\(([^)]+)\)", text):
        target = m.group(1).split("#", 1)[0]
        if not target or target.startswith("http") or target.startswith("mailto:"):
            continue
        full = os.path.normpath(os.path.join(docs, target))
        if not os.path.exists(full):
            print(f"broken link in {name}: {target}")
            sys.exit(1)
print("links ok")
PY

# Safe commands against the built binary.
rustup run stable cargo build -p bootlogo-forge >/dev/null
BIN="$ROOT/target/debug/bootlogo-forge"
test -x "$BIN"

HELP="$("$BIN" --help)"
printf '%s\n' "$HELP" | grep -q "Replace the boot logo via ACPI BGRT" || {
    echo "help output changed"
    exit 1
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/esp/EFI/ubuntu"
printf x > "$TMP/esp/EFI/ubuntu/shimx64.efi"
printf 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGPgEpEDAABoAD1UCKP3AAAAAElFTkSuQmCC' | base64 -d > "$TMP/logo.png"
touch "$TMP/efi.efi"

"$BIN" install --image "$TMP/logo.png" --esp "$TMP/esp" --efi-app "$TMP/efi.efi" | grep -q "DRY RUN" || {
    echo "dry run output changed"
    exit 1
}

mkdir -p "$TMP/dmi"
printf 'Acer\n' > "$TMP/dmi/sys_vendor"
printf 'Example Model\n' > "$TMP/dmi/product_name"
"$BIN" detect --dmi-dir "$TMP/dmi" >/dev/null

echo "safe commands ok"