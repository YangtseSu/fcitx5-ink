#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
import json
import os
import socket
import sys

path = os.path.join(
    os.environ["XDG_RUNTIME_DIR"], "fcitx5-ink/panel-input.sock"
)
request = json.loads(sys.argv[1])
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
    connection.settimeout(3)
    connection.connect(path)
    connection.sendall((json.dumps(request, ensure_ascii=False) + "\n").encode())
    reply = connection.makefile("rb").readline().decode().strip()
print(reply)
if json.loads(reply).get("ok") is not True:
    raise SystemExit(1)
