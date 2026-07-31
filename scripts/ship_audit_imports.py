#!/usr/bin/env python3
"""Ship audit — read a Windows PE's import tables so the depot doc can list the
runtime DLLs honestly instead of guessing from `strings`.

Prints, for the given exe:
  * IMPORT     — DLLs the loader resolves at process start (missing one = the exe
                 refuses to launch with 0xc0000135 before any of our code runs)
  * DELAY      — delay-load imports (resolved on first use, not at boot)

No third-party modules: this parses the PE headers directly so it runs on the
same bare Python the other scripts/ tools use.

Usage:  python scripts/ship_audit_imports.py target/release/voxelforge.exe
"""

import struct
import sys


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def parse(path):
    with open(path, "rb") as f:
        data = f.read()

    if data[:2] != b"MZ":
        raise SystemExit(f"{path}: not a PE image (no MZ)")
    pe = u32(data, 0x3C)
    if data[pe:pe + 4] != b"PE\0\0":
        raise SystemExit(f"{path}: no PE signature at e_lfanew")

    n_sections = u16(data, pe + 6)
    opt_size = u16(data, pe + 20)
    opt = pe + 24
    magic = u16(data, opt)
    pe32plus = magic == 0x20B
    # Data directories start after the optional header's fixed part.
    dd = opt + (112 if pe32plus else 96)

    sections = []
    sec = opt + opt_size
    for i in range(n_sections):
        off = sec + i * 40
        sections.append((
            u32(data, off + 12),   # VirtualAddress
            u32(data, off + 8),    # VirtualSize
            u32(data, off + 20),   # PointerToRawData
            u32(data, off + 16),   # SizeOfRawData
        ))

    def rva2off(rva):
        for va, vsize, praw, sraw in sections:
            if va <= rva < va + max(vsize, sraw):
                return praw + (rva - va)
        return None

    def cstr(rva):
        o = rva2off(rva)
        if o is None:
            return None
        end = data.index(b"\0", o)
        return data[o:end].decode("ascii", "replace")

    def dir_entry(idx):
        return u32(data, dd + idx * 8), u32(data, dd + idx * 8 + 4)

    out = {}

    # ---- directory 1: import table -----------------------------------------
    imp_rva, imp_size = dir_entry(1)
    names = []
    if imp_rva:
        o = rva2off(imp_rva)
        while True:
            name_rva = u32(data, o + 12)
            if name_rva == 0 and u32(data, o) == 0:
                break
            n = cstr(name_rva)
            if not n:
                break
            names.append(n)
            o += 20
    out["IMPORT"] = names

    # ---- directory 13: delay-load import table ------------------------------
    dly_rva, _ = dir_entry(13)
    dnames = []
    if dly_rva:
        o = rva2off(dly_rva)
        while True:
            attrs = u32(data, o)
            name_rva = u32(data, o + 4)
            if name_rva == 0:
                break
            # attrs bit0 set => name_rva is an RVA (modern linkers); else a VA.
            rva = name_rva if (attrs & 1) else None
            n = cstr(rva) if rva is not None else None
            if not n:
                break
            dnames.append(n)
            o += 32
    out["DELAY"] = dnames
    return out


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    for path in sys.argv[1:]:
        print(f"=== {path} ===")
        tables = parse(path)
        for kind in ("IMPORT", "DELAY"):
            for n in sorted(set(tables[kind]), key=str.lower):
                print(f"  {kind:6} {n}")
            if not tables[kind]:
                print(f"  {kind:6} (none)")


if __name__ == "__main__":
    main()
