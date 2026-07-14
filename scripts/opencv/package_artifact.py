#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
import zipfile
from datetime import datetime, timezone
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def mib(size: int) -> str:
    return f"{size / 1024 / 1024:.2f} MiB"


def main() -> int:
    parser = argparse.ArgumentParser(description="Package a minimal OpenCV installation and report its size")
    parser.add_argument("--install", required=True, type=Path)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--name", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--modules", required=True)
    args = parser.parse_args()

    install = args.install.resolve()
    if not install.is_dir():
        raise SystemExit(f"OpenCV install directory does not exist: {install}")

    license_source = args.source.resolve() / "LICENSE"
    if not license_source.is_file():
        raise SystemExit(f"OpenCV license was not found: {license_source}")
    shutil.copy2(license_source, install / "LICENSE-OpenCV.txt")

    libraries = []
    for path in install.rglob("*"):
        if path.is_file() and path.suffix.lower() in {".a", ".lib"}:
            libraries.append({
                "path": path.relative_to(install).as_posix(),
                "bytes": path.stat().st_size,
                "sha256": sha256(path),
            })
    libraries.sort(key=lambda item: item["bytes"], reverse=True)
    if not libraries:
        raise SystemExit("No static libraries were found in the OpenCV installation")
    dynamic_libraries = [
        path.relative_to(install).as_posix()
        for path in install.rglob("*")
        if path.is_file()
        and (path.suffix.lower() in {".dll", ".dylib"} or ".so" in path.name.lower())
    ]
    if dynamic_libraries:
        raise SystemExit(f"Unexpected dynamic libraries found: {', '.join(dynamic_libraries)}")

    modules = [module.strip() for module in args.modules.split(",") if module.strip()]
    library_names = [Path(library["path"]).name.lower() for library in libraries]
    missing_modules = [
        module for module in modules
        if not any(f"opencv_{module}" in library_name for library_name in library_names)
    ]
    if missing_modules:
        raise SystemExit(f"Static libraries missing required modules: {', '.join(missing_modules)}")

    manifest = {
        "schemaVersion": 1,
        "name": args.name,
        "opencvVersion": args.version,
        "opencvCommit": args.commit,
        "target": args.target,
        "modules": modules,
        "generatedAt": datetime.now(timezone.utc).isoformat(),
        "libraries": libraries,
    }
    manifest_path = install / "pixelforge-opencv-manifest.json"
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    files = sorted(path for path in install.rglob("*") if path.is_file())
    uncompressed_size = sum(path.stat().st_size for path in files)
    static_library_size = sum(library["bytes"] for library in libraries)
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f"{args.name}-{args.version}.zip"
    root_name = f"{args.name}-{args.version}"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
        for path in files:
            output.write(path, (Path(root_name) / path.relative_to(install)).as_posix())

    archive_size = archive.stat().st_size
    archive_hash = sha256(archive)
    checksum_path = archive.with_suffix(archive.suffix + ".sha256")
    checksum_path.write_text(f"{archive_hash}  {archive.name}\n", encoding="utf-8")

    report_lines = [
        f"## {args.name}",
        "",
        f"- OpenCV: `{args.version}` (`{args.commit}`)",
        f"- Target: `{args.target}`",
        f"- Modules: `{args.modules}`",
        f"- Installed files: `{len(files)}`",
        f"- Static libraries combined: **{mib(static_library_size)}** (`{static_library_size}` bytes)",
        f"- Uncompressed installation: **{mib(uncompressed_size)}** (`{uncompressed_size}` bytes)",
        f"- ZIP archive: **{mib(archive_size)}** (`{archive_size}` bytes)",
        f"- SHA-256: `{archive_hash}`",
        "",
        "### Static libraries",
        "",
        "| Library | Size | Bytes |",
        "|---|---:|---:|",
    ]
    for library in libraries:
        report_lines.append(f"| `{library['path']}` | {mib(library['bytes'])} | {library['bytes']} |")
    report_lines.append("")
    report = "\n".join(report_lines)
    report_path = args.output / f"{args.name}-{args.version}-size-report.md"
    report_path.write_text(report, encoding="utf-8")

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as summary:
            summary.write(report)
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
