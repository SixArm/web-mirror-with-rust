#!/bin/sh
# Benchmark web-mirror against wget (and wget2, if installed) on a local site.
# Needs: python3, hyperfine, wget; a release build (cargo build --release).
# Usage: spec/compare/benchmarks/run.sh [PAGES] [LATENCY_MS]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
pages=${1:-200}
latency=${2:-20}
port=8766
url="http://127.0.0.1:$port/docs/"
target=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
wm="$target/release/web-mirror"
work=$(mktemp -d)
python3 -I "$here/server.py" "$port" "$pages" "$latency" &
server=$!
trap 'kill $server; rm -rf "$work"' EXIT
sleep 1
cd "$work"
set -- -n "wget -r -np" "wget -q -r -np -l inf -P out $url"
if command -v wget2 >/dev/null; then
  set -- "$@" -n "wget2 -r -np" "wget2 -q -r -np -l 0 -P out $url" \
    -n "wget2 -r -np --max-threads 16" "wget2 -q -r -np -l 0 --max-threads 16 -P out $url"
fi
hyperfine -N --runs 5 --prepare 'rm -rf out' "$@" \
  -n "web-mirror" "$wm $url -P out -v" \
  -n "web-mirror --max-connections 4 --per-host 4" "$wm $url -P out -v --max-connections 4 --per-host 4" \
  -n "web-mirror --max-connections 16 --per-host 16" "$wm $url -P out -v --max-connections 16 --per-host 16"
for cmd in "wget -q -r -np -l inf -P out $url" "$wm $url -P out -v" "$wm $url -P out -v --max-connections 16 --per-host 16"; do
  rm -rf out
  printf '%s: ' "$cmd"
  /usr/bin/time -l $cmd 2>&1 | grep 'maximum resident' || true
done
