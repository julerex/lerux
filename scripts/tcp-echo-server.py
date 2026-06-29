#!/usr/bin/env python3
"""Background TCP echo server for virtio net smoke tests."""

import socket
import sys
import threading


def port_is_listening(port: int) -> bool:
    probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        probe.bind(("127.0.0.1", port))
    except OSError:
        return True
    finally:
        probe.close()
    return False


def serve(port: int, done: threading.Event) -> None:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("127.0.0.1", port))
    sock.listen(5)
    print(f"tcp-echo-server: listening on 127.0.0.1:{port}", file=sys.stderr, flush=True)
    while not done.is_set():
        conn, _ = sock.accept()
        data = conn.recv(64)
        if data:
            conn.sendall(data)
        conn.close()
        done.set()
    sock.close()


def main() -> None:
    port = int(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1] != "--probe" else 18080
    if len(sys.argv) > 1 and sys.argv[1] == "--probe":
        port = int(sys.argv[2]) if len(sys.argv) > 2 else 18080
        sys.exit(0 if port_is_listening(port) else 1)

    done = threading.Event()
    thread = threading.Thread(target=serve, args=(port, done), daemon=True)
    thread.start()
    thread.join(timeout=60)


if __name__ == "__main__":
    main()