#!/usr/bin/env python3
"""Package a directory as a deterministic ZIP archive."""

import argparse
import stat
import zipfile
from pathlib import Path


FIXED_TIMESTAMP = (1980, 1, 1, 0, 0, 0)
FILE_MODE = stat.S_IFREG | 0o644


def source_files(root):
    return sorted(path for path in root.rglob("*") if path.is_file())


def archive_info(path, root):
    info = zipfile.ZipInfo(path.relative_to(root).as_posix(), FIXED_TIMESTAMP)
    info.compress_type = zipfile.ZIP_DEFLATED
    info.create_system = 3
    info.external_attr = FILE_MODE << 16
    return info


def package(root, target):
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in source_files(root):
            archive.writestr(archive_info(path, root), path.read_bytes(), compresslevel=9)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    package(args.source, args.output)


if __name__ == "__main__":
    main()
