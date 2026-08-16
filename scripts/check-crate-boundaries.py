#!/usr/bin/env python3
"""Fail if workspace crate dependencies violate docs/TRD.md boundaries."""

from __future__ import annotations

import json
import subprocess
import sys
from collections import defaultdict, deque


FORBIDDEN = {
    "circulo-app": {"circulo-adapter-opencode", "circulo-persist"},
    "circulo-adapter-opencode": {"circulo-app"},
    "circulo-adapter-fake": {"circulo-app"},
    "circulo-protocol": {
        "circulo-app",
        "circulo-daemon",
        "circulo-adapter",
        "circulo-adapter-fake",
        "circulo-adapter-opencode",
        "circulo-persist",
    },
}


def cargo_metadata() -> dict:
    raw = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--offline"],
        stderr=subprocess.DEVNULL,
    )
    return json.loads(raw)


def package_name(pkg_id: str, id_to_name: dict[str, str]) -> str:
    return id_to_name[pkg_id]


def main() -> int:
    meta = cargo_metadata()
    id_to_name = {pkg["id"]: pkg["name"] for pkg in meta["packages"]}
    workspace_names = {
        id_to_name[pkg_id] for pkg_id in meta["workspace_members"]
    }

    graph: dict[str, set[str]] = defaultdict(set)
    for node in meta["resolve"]["nodes"]:
        src = id_to_name.get(node["id"])
        if src not in workspace_names:
            continue
        for dep in node.get("deps", []):
            dst = dep["name"]
            if dst in workspace_names:
                graph[src].add(dst)

    errors: list[str] = []
    for src, banned in FORBIDDEN.items():
        seen: set[str] = set()
        queue = deque(graph.get(src, ()))
        while queue:
            cur = queue.popleft()
            if cur in seen:
                continue
            seen.add(cur)
            if cur in banned:
                errors.append(f"{src} must not depend on {cur} (direct or transitive)")
            queue.extend(graph.get(cur, ()))

    if errors:
        print("crate boundary check failed:", file=sys.stderr)
        for line in errors:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("crate boundary check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
