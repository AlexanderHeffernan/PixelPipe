#!/usr/bin/python3
"""Deterministic CC0 fake for the pixelpipe.agent-request/v1 protocol."""

import hashlib
import json
import os
import struct
import sys
import time
import zlib


def png_chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def synthetic_png():
    rows = b"\x00\xff\x40\x20\xff\x00\x00\x00\x00" + b"\x00\x20\x80\xff\xff\xff\xd0\x50\xff"
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 6, 0, 0, 0))
        + png_chunk(b"IDAT", zlib.compress(rows, 9))
        + png_chunk(b"IEND", b"")
    )


request = json.load(sys.stdin)
mode = sys.argv[1] if len(sys.argv) > 1 else "success"

if mode == "cancel":
    time.sleep(10)
if mode == "exit-failure":
    print("intentional fake failure", file=sys.stderr)
    sys.exit(7)
if mode == "malformed":
    print("not-json")
    sys.exit(0)

secret = os.environ.get("HOME", "")
print(f"fake progress secret={secret}", file=sys.stderr, flush=True)

identity = {
    "adapter": "pixelpipe-fixture-agent",
    "provider": "fixture-provider",
    "model": "fixture-model",
    "capabilities": ["generate_references", "critique_asset", "propose_refinement"],
}
operation = request["operation"]

if operation == "generate_references":
    output = request["workspace"]["output_directory"]
    if mode == "escape":
        path = os.path.join(output, "..", "escaped.png")
        returned_path = "../escaped.png"
    else:
        path = os.path.join(output, "candidate.png")
        returned_path = "candidate.png"
    image = synthetic_png()
    with open(path, "wb") as candidate:
        candidate.write(image)
    if mode == "cancel-partial":
        print('{"schema":"pixelpipe.agent-response/v1","adapter":', end="", flush=True)
        time.sleep(10)
    digest = hashlib.sha256(image).hexdigest()
    if mode == "bad-hash":
        digest = "0" * 64
    candidates = [{"id": "candidate-one", "path": returned_path, "sha256": digest}]
    if mode == "later-bad-hash":
        with open(os.path.join(output, "candidate-two.png"), "wb") as candidate:
            candidate.write(image)
        candidates.append({"id": "candidate-two", "path": "candidate-two.png", "sha256": "0" * 64})
    result = {
        "type": "generated_references",
        "candidates": candidates,
    }
elif operation == "critique_asset":
    result = {"type": "critique", "text": f"Silhouette reads clearly. secret={secret}"}
else:
    result = {
        "type": "proposal",
        "proposal": {
            "type": "pixel_patch",
            "patch": {
                "schema": "pixelpipe.patch/v1",
                "edits": [{"x": 2, "y": 0, "index": 2}],
            },
        },
    }

json.dump({"schema": "pixelpipe.agent-response/v1", "adapter": identity, "result": result}, sys.stdout)
