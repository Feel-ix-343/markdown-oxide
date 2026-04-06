#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

DEFAULT_FPF_SPEC_URL = (
    "https://raw.githubusercontent.com/ailev/FPF/main/FPF-Spec.md"
)


def fetch_fpf_spec(output_dir: Path, url: str) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    destination = output_dir / "FPF-Spec.md"

    with urllib.request.urlopen(url) as response, destination.open("wb") as file:
        shutil.copyfileobj(response, file)

    return destination


def send_message(process: subprocess.Popen[bytes], message: dict[str, object]) -> None:
    body = json.dumps(message).encode("utf-8")
    header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
    assert process.stdin is not None
    process.stdin.write(header + body)
    process.stdin.flush()


def read_message(process: subprocess.Popen[bytes]) -> dict[str, object]:
    assert process.stdout is not None

    headers: dict[str, str] = {}
    while True:
        line = process.stdout.readline()
        if not line:
            raise EOFError("unexpected EOF while reading LSP headers")
        if line == b"\r\n":
            break
        key, value = line.decode("ascii").split(":", 1)
        headers[key.strip().lower()] = value.strip()

    length = int(headers["content-length"])
    return json.loads(process.stdout.read(length))


def benchmark_initialize(
    binary: Path,
    markdown_file: Path,
    timeout_seconds: float,
) -> tuple[float, list[str]]:
    with tempfile.TemporaryDirectory(prefix="markdown-oxide-large-file-") as temp_dir:
        vault_root = Path(temp_dir)
        vault_file = vault_root / markdown_file.name
        shutil.copy2(markdown_file, vault_file)

        process = subprocess.Popen(
            [str(binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        try:
            send_message(
                process,
                {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "processId": None,
                        "rootUri": vault_root.resolve().as_uri(),
                        "capabilities": {},
                        "workspaceFolders": [
                            {
                                "uri": vault_root.resolve().as_uri(),
                                "name": vault_root.name,
                            }
                        ],
                        "clientInfo": {
                            "name": "large-file-benchmark",
                            "version": "1",
                        },
                        "initializationOptions": None,
                    },
                },
            )

            start = time.perf_counter()
            logs: list[str] = []

            while True:
                if time.perf_counter() - start > timeout_seconds:
                    raise TimeoutError(
                        f"initialize did not finish within {timeout_seconds} seconds"
                    )

                message = read_message(process)
                if message.get("method") == "window/logMessage":
                    params = message.get("params", {})
                    if isinstance(params, dict):
                        log_message = params.get("message")
                        if isinstance(log_message, str):
                            logs.append(log_message)

                if message.get("id") == 1:
                    if "error" in message:
                        raise RuntimeError(f"initialize failed: {message['error']}")
                    elapsed = time.perf_counter() - start
                    return elapsed, logs
        finally:
            try:
                send_message(process, {"jsonrpc": "2.0", "method": "exit", "params": {}})
            except Exception:
                pass

            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)


def fetch_command(args: argparse.Namespace) -> int:
    destination = fetch_fpf_spec(args.output_dir, args.url)
    print(destination)
    return 0


def run_command(args: argparse.Namespace) -> int:
    binary = args.binary.resolve()
    markdown_file = args.file.resolve()

    if not binary.exists():
        raise FileNotFoundError(f"binary does not exist: {binary}")
    if not markdown_file.exists():
        raise FileNotFoundError(f"markdown file does not exist: {markdown_file}")

    elapsed, logs = benchmark_initialize(binary, markdown_file, args.timeout_seconds)

    print(f"binary={binary}")
    print(f"markdown_file={markdown_file}")
    print(f"file_size_bytes={markdown_file.stat().st_size}")
    print(f"initialize_seconds={elapsed:.3f}")

    for log in logs:
        print(log)

    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Fetch FPF-Spec.md and benchmark markdown-oxide on a one-file vault."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    fetch_parser = subparsers.add_parser(
        "fetch",
        help="Download FPF-Spec.md into a stable directory outside the git checkout.",
    )
    fetch_parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="Directory that will receive FPF-Spec.md.",
    )
    fetch_parser.add_argument(
        "--url",
        default=DEFAULT_FPF_SPEC_URL,
        help=f"Source URL (default: {DEFAULT_FPF_SPEC_URL}).",
    )
    fetch_parser.set_defaults(func=fetch_command)

    run_parser = subparsers.add_parser(
        "run",
        help="Benchmark raw LSP initialize time against a one-file vault.",
    )
    run_parser.add_argument(
        "--binary",
        type=Path,
        required=True,
        help="Path to the markdown-oxide release binary.",
    )
    run_parser.add_argument(
        "--file",
        type=Path,
        required=True,
        help="Path to FPF-Spec.md (or another large markdown file).",
    )
    run_parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=180.0,
        help="Fail if initialize takes longer than this many seconds.",
    )
    run_parser.set_defaults(func=run_command)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
