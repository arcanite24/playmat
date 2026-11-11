#!/usr/bin/env bash
# Runs locally what CI would run for the files this branch changes, so a PR doesn't bounce on a check that could have
# failed here first. The pre-push hook (.githooks/pre-push) calls it; you can also run it by hand.
#
#   ops/ci/check.sh                 check the files changed since the branch left master (plus uncommitted ones)
#   ops/ci/check.sh --java-tests    also run the Java tests of the changed modules and their dependents
#   ops/ci/check.sh --web-build     also run the web client's build and bundle-size check
#   ops/ci/check.sh --e2e           also run the web client's Playwright specs (implies --web-build)
#   ops/ci/check.sh --full          all of the above
#   ops/ci/check.sh --all           check every area, whatever changed
#   ops/ci/check.sh --list          only print what would run
#   ops/ci/check.sh --base <ref>    compare against <ref> instead of fork/master (origin/master, master)
#
# What runs, by changed path (the same commands as .github/workflows):
#   rust/**                     cargo fmt --all --check; cargo clippy --workspace --all-targets --locked -- -D warnings;
#                               cargo test --workspace --locked                                         (rust.yml)
#   Mage.Web.Client/**          npm run typecheck; npm run lint; npm test (npm ci first when needed)  (web-client.yml)
#   Java sources or pom.xml     mvn -o -T 1C test-compile of the changed modules and what they need     (maven.yml)
#   .github/workflows/*.yml     actionlint on the changed workflows (syntax and expressions; its shellcheck pass is
#                               off, CI doesn't run one), when it is installed
#
# Exits non-zero when a check fails. To push anyway: git push --no-verify
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || { echo "check.sh: not inside a git checkout" >&2; exit 2; }
cd "$ROOT" || exit 2

JAVA_TESTS=0 WEB_BUILD=0 E2E=0 ALL=0 LIST=0 BASE="" REV="" FROM_HOOK=0
while [ $# -gt 0 ]; do
  case "$1" in
    --java-tests) JAVA_TESTS=1 ;;
    --web-build) WEB_BUILD=1 ;;
    --e2e) E2E=1; WEB_BUILD=1 ;;
    --full) JAVA_TESTS=1; WEB_BUILD=1; E2E=1 ;;
    --all) ALL=1 ;;
    --list) LIST=1 ;;
    --base) BASE=${2:?--base needs a ref}; shift ;;
    # the commit to check (the hook passes the one being pushed); only committed changes count then
    --rev) REV=${2:?--rev needs a commit}; shift ;;
    --hook) FROM_HOOK=1 ;;
    -h|--help) sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "check.sh: unknown option $1 (see --help)" >&2; exit 2 ;;
  esac
  shift
done

if [ -t 1 ]; then
  BOLD=$'\033[1m' RED=$'\033[31m' GREEN=$'\033[32m' YELLOW=$'\033[33m' DIM=$'\033[2m' RESET=$'\033[0m'
else
  BOLD="" RED="" GREEN="" YELLOW="" DIM="" RESET=""
fi
say() { printf '%s\n' "$*"; }
note() { printf '%s%s%s\n' "$DIM" "$*" "$RESET"; }

# ---- what changed --------------------------------------------------------------------------------------------------
if [ -z "$BASE" ]; then
  for candidate in fork/master origin/master master; do
    if git rev-parse --verify -q "$candidate^{commit}" >/dev/null; then BASE=$candidate; break; fi
  done
fi
[ -n "$BASE" ] || { echo "check.sh: no fork/master, origin/master or master to compare with; pass --base <ref>" >&2; exit 2; }
TARGET=${REV:-HEAD}
MERGE_BASE=$(git merge-base "$BASE" "$TARGET" 2>/dev/null) || { echo "check.sh: $BASE and $TARGET share no history" >&2; exit 2; }

CHANGED=$(git diff --name-only "$MERGE_BASE" "$TARGET")
if [ -z "$REV" ]; then
  # by hand: count uncommitted and untracked files too, they are what the checks see
  CHANGED=$(printf '%s\n%s\n%s\n' "$CHANGED" "$(git diff --name-only HEAD)" "$(git ls-files --others --exclude-standard)")
fi
CHANGED=$(printf '%s\n' "$CHANGED" | sed '/^$/d' | sort -u)

has() { printf '%s\n' "$CHANGED" | grep -Eq "$1"; }

DO_RUST=0 DO_WEB=0 DO_JAVA=0 DO_WORKFLOWS=0
if [ "$ALL" = 1 ]; then
  DO_RUST=1 DO_WEB=1 DO_JAVA=1 DO_WORKFLOWS=1
else
  # the same paths as rust.yml and web-client.yml
  has '^(rust/|Mage\.Web\.Client/src/core/rpc/__fixtures__/gameStatePatches\.json$|\.github/workflows/rust\.yml$)' && DO_RUST=1
  has '^(Mage\.Web\.Client/|\.github/workflows/web-client\.yml$)' && DO_WEB=1
  # Java: sources, resources and poms of the Maven modules (not Mage.Web.Client, and not e.g. Mage.Server/career-src)
  has '^(pom\.xml$|\.travis/|Mage[^/]*/(.*/)?(pom\.xml$|src/))' && DO_JAVA=1
  has '^\.github/workflows/' && DO_WORKFLOWS=1
fi
# Maven modules of the changed Java files: the nearest folder with a pom.xml (the root pom means every module)
JAVA_MODULES=""
if [ "$DO_JAVA" = 1 ] && [ "$ALL" = 0 ]; then
  while IFS= read -r file; do
    case "$file" in
      pom.xml|.travis/*) JAVA_MODULES="*"; break ;;
      Mage.Web.Client/*) continue ;;
      Mage*/*) ;;
      *) continue ;;
    esac
    case "$file" in */pom.xml|*/src/*) ;; *) continue ;; esac
    dir=$(dirname "$file")
    while [ "$dir" != "." ] && [ ! -f "$dir/pom.xml" ]; do dir=$(dirname "$dir"); done
    [ "$dir" = "." ] && { JAVA_MODULES="*"; break; }
    case ",$JAVA_MODULES," in *",$dir,"*) ;; *) JAVA_MODULES=${JAVA_MODULES:+$JAVA_MODULES,}$dir ;; esac
  done <<EOF
$CHANGED
EOF
  [ -z "$JAVA_MODULES" ] && DO_JAVA=0
elif [ "$DO_JAVA" = 1 ]; then
  JAVA_MODULES="*"
fi

COUNT=$(printf '%s\n' "$CHANGED" | sed '/^$/d' | wc -l | tr -d ' ')
say "${BOLD}Playmat checks${RESET} ${DIM}($COUNT files changed vs $BASE at $(git rev-parse --short "$MERGE_BASE"))${RESET}"

if [ $((DO_RUST + DO_WEB + DO_JAVA + DO_WORKFLOWS)) = 0 ]; then
  say "Nothing CI checks changed (Rust, web client, Java, workflows): skipping."
  exit 0
fi

if [ -z "$REV" ] || [ "$(git rev-parse "$REV")" = "$(git rev-parse HEAD)" ]; then
  if [ -n "$(git status --porcelain --untracked-files=no)" ] && [ "$FROM_HOOK" = 1 ]; then
    say "${YELLOW}Note: uncommitted changes; the checks run on the working tree, not only on what you push.${RESET}"
  fi
fi

# ---- the steps -----------------------------------------------------------------------------------------------------
STEPS=()        # "area|label|dir|command"
step() { STEPS+=("$1|$2|$3|$4"); }

if [ "$DO_RUST" = 1 ]; then
  step rust "cargo fmt --check" rust "cargo fmt --all --check"
  step rust "cargo clippy -D warnings" rust "cargo clippy --workspace --all-targets --locked -- -D warnings"
  step rust "cargo test" rust "cargo test --workspace --locked"
fi
if [ "$DO_WEB" = 1 ]; then
  step web "npm ci (if needed)" Mage.Web.Client "__npm_ci_if_needed"
  step web "typecheck" Mage.Web.Client "npm run typecheck"
  step web "lint" Mage.Web.Client "npm run lint"
  step web "unit and a11y tests" Mage.Web.Client "npm test"
  if [ "$WEB_BUILD" = 1 ]; then
    step web "build" Mage.Web.Client "npm run build"
    step web "bundle size" Mage.Web.Client "npm run size"
  fi
  [ "$E2E" = 1 ] && step web "Playwright e2e" Mage.Web.Client "npm run test:e2e:ci"
fi
if [ "$DO_JAVA" = 1 ]; then
  if [ "$JAVA_MODULES" = "*" ]; then PL=""; else PL="-pl $JAVA_MODULES -am"; fi
  step java "compile${PL:+ ($JAVA_MODULES)}" . "__mvn -q -T 1C ${PL:+$PL }test-compile"
  if [ "$JAVA_TESTS" = 1 ]; then
    [ -n "$PL" ] && PL="-pl $JAVA_MODULES -am -amd"
    step java "tests" . "__mvn -B -T 1C ${PL:+$PL }test -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$ROOT/.travis/log4j.properties"
  fi
fi
if [ "$DO_WORKFLOWS" = 1 ]; then
  if command -v actionlint >/dev/null 2>&1; then
    if [ "$ALL" = 1 ]; then
      WORKFLOWS=".github/workflows/*.yml"
    else
      WORKFLOWS=$(printf '%s\n' "$CHANGED" | grep -E '^\.github/workflows/.*\.ya?ml$' | while read -r f; do [ -f "$f" ] && printf '%s ' "$f"; done)
    fi
    [ -n "$WORKFLOWS" ] && step workflows "actionlint" . "actionlint -shellcheck= $WORKFLOWS"
  else
    note "workflows changed, but actionlint isn't installed (brew install actionlint): not linted"
  fi
fi

if [ "$LIST" = 1 ]; then
  for s in "${STEPS[@]}"; do
    IFS='|' read -r area label dir cmd <<<"$s"
    cmd=${cmd/__npm_ci_if_needed/npm ci   # only when node_modules is missing or older than package-lock.json}
    cmd=${cmd/__mvn/mvn -o}
    say "  [$area] $label: ${DIM}(cd $dir && $cmd)${RESET}"
  done
  exit 0
fi

npm_ci_if_needed() {
  if [ ! -f node_modules/.package-lock.json ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
    npm ci --no-audit --no-fund
  else
    echo "node_modules is up to date"
  fi
}

# offline first (fast, no network); online again when the local repository lacks something
mvn_offline_first() {
  local log
  log=$(mktemp "${TMPDIR:-/tmp}/playmat-check-mvn.XXXXXX")
  mvn -o "$@" 2>&1 | tee "$log"
  local status=${PIPESTATUS[0]}
  if [ "$status" != 0 ] && grep -Eq 'offline mode|has not been downloaded|Cannot access .* in offline' "$log"; then
    echo "(missing from the local Maven repository: retrying online)"
    mvn "$@"
    status=$?
  fi
  rm -f "$log"
  return "$status"
}

FAILED=() RESULTS=() BROKEN_AREA=""
for s in "${STEPS[@]}"; do
  IFS='|' read -r area label dir cmd <<<"$s"
  if [ "$area" = "$BROKEN_AREA" ]; then
    RESULTS+=("${YELLOW}skip${RESET} [$area] $label ${DIM}(npm ci failed)${RESET}")
    continue
  fi
  say ""
  say "${BOLD}==> [$area] $label${RESET} ${DIM}($dir)${RESET}"
  start=$(date +%s)
  case "$cmd" in
    __npm_ci_if_needed) (cd "$dir" && npm_ci_if_needed) </dev/null ;;
    __mvn*) (cd "$dir" && eval "mvn_offline_first ${cmd#__mvn }") </dev/null ;;
    *) (cd "$dir" && eval "$cmd") </dev/null ;;
  esac
  status=$?
  secs=$(( $(date +%s) - start ))
  if [ "$status" = 0 ]; then
    RESULTS+=("${GREEN}ok  ${RESET} [$area] $label ${DIM}${secs}s${RESET}")
  else
    RESULTS+=("${RED}FAIL${RESET} [$area] $label ${DIM}${secs}s${RESET}")
    rerun=${cmd/__mvn/mvn -o}
    rerun=${rerun/__npm_ci_if_needed/npm ci}
    FAILED+=("[$area] $label: (cd $dir && $rerun)")
    # the other web steps can't run without node_modules
    [ "$cmd" = __npm_ci_if_needed ] && BROKEN_AREA=$area
  fi
done

say ""
say "${BOLD}Summary${RESET}"
for r in "${RESULTS[@]}"; do say "  $r"; done
if [ ${#FAILED[@]} -gt 0 ]; then
  say ""
  say "${RED}${BOLD}${#FAILED[@]} check(s) failed${RESET}; CI would fail the same way. Rerun one with:"
  for f in "${FAILED[@]}"; do say "  $f"; done
  [ "$FROM_HOOK" = 1 ] && say "To push anyway: ${BOLD}git push --no-verify${RESET}"
  exit 1
fi
say "${GREEN}All checks passed.${RESET}"
