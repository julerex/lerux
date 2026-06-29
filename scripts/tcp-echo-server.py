#!/usr/bin/env python3
"""Background TCP echo server for virtio net smoke tests."""

import socket
import sys
import threading


def serve(port: int) -> None:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("127.0.0.1", port))
    sock.listen(1)
    conn, _ = sock.accept()
    data = conn.recv(64)
    conn.sendall(data)
    conn.close()
    sock.close()


def main() -> None:
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 18080
    thread = threading.Thread(target=serve, args=(port,), daemon=True)
    thread.start()
    thread.join(timeout=30)


if __name__ == "__main__":
    main()