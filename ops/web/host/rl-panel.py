#!/usr/bin/env python3
"""The self-play worker's side of the admin console's files (rl-worker.sh, SelfPlayStatus.java), in the compose
volume xmage-selfplay. The worker runs as root, but the volume belongs to the server's container user, so nothing in
it is trusted: a planted symlink or FIFO must not make root write, chmod or read anything else. So this opens the
volume's directory without following links, drops to the directory's owner, and does every read and write relative
to that handle with O_NOFOLLOW. The trainer's file is read from the spool as root first, the same careful way.

    playmat-rl-panel settings PANEL            settings.json's fields, one "key<TAB>value" a line
    playmat-rl-panel write PANEL worker.json   stdin (one JSON object) becomes the file, atomically
    playmat-rl-panel append PANEL              stdin (one JSON object) is a new line of history.jsonl (5000 kept)
    playmat-rl-panel trainer PANEL FILE        copies the trainer's progress (FILE, in the spool) in when it changed
"""

import json
import os
import stat
import sys

CAP = 65536
HISTORY = "history.jsonl"
HISTORY_KEEP = 5000
NAMES = ("worker.json", "trainer.json")


def read_file(name, dir_fd=None, cap=CAP):
    """A regular file's bytes, or None: no link followed, no FIFO waited on, nothing over cap."""
    try:
        f = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=dir_fd)
    except OSError:
        return None
    try:
        if not stat.S_ISREG(os.fstat(f).st_mode):
            return None
        data = b""
        while len(data) <= cap:
            part = os.read(f, cap + 1 - len(data))
            if not part:
                break
            data += part
        return data if len(data) <= cap else None
    finally:
        os.close(f)


def open_panel(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    st = os.fstat(fd)
    if os.geteuid() == 0:
        if st.st_uid == 0:
            sys.exit("the panel directory is root's: not the server's volume")
        os.setgroups([])
        os.setgid(st.st_gid)
        os.setuid(st.st_uid)
    os.umask(0o022)
    return fd


def write_atomic(fd, name, data):
    tmp = f".{name}.{os.getpid()}.tmp"
    try:
        os.unlink(tmp, dir_fd=fd)
    except FileNotFoundError:
        pass
    f = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644, dir_fd=fd)
    with os.fdopen(f, "wb") as out:
        os.fchmod(out.fileno(), 0o644)
        out.write(data)
    # rename replaces whatever is at name (a link included) without following it
    os.rename(tmp, name, src_dir_fd=fd, dst_dir_fd=fd)


def json_object(data):
    try:
        j = json.loads(data.decode("utf-8"))
    except (ValueError, UnicodeDecodeError, AttributeError):
        return None
    return j if isinstance(j, dict) else None


def stdin_object():
    data = sys.stdin.buffer.read(CAP + 1)
    j = json_object(data) if len(data) <= CAP else None
    if j is None:
        sys.exit("stdin must be one JSON object of at most 64 KB")
    return j


def main(argv):
    if len(argv) < 3:
        sys.exit(__doc__)
    mode, panel = argv[1], argv[2]
    if mode == "settings":
        fd = open_panel(panel)
        j = json_object(read_file("settings.json", fd, 4096) or b"")
        for k in ("paused", "quiet_utc", "cpu_max", "games_max", "jobs"):
            v = (j or {}).get(k)
            if isinstance(v, bool):
                v = "true" if v else "false"
            elif isinstance(v, (int, str)):
                v = str(v)
            else:
                continue
            if "\t" not in v and "\n" not in v:
                print(k + "\t" + v)
    elif mode == "write" and len(argv) == 4 and argv[3] in NAMES:
        j = stdin_object()
        fd = open_panel(panel)
        write_atomic(fd, argv[3], json.dumps(j).encode())
    elif mode == "append":
        j = stdin_object()
        fd = open_panel(panel)
        line = (json.dumps(j) + "\n").encode()
        old = read_file(HISTORY, fd, 16 << 20)
        if old is None:
            old = b""
        lines = old.splitlines(keepends=True)
        if len(lines) >= HISTORY_KEEP:
            write_atomic(fd, HISTORY, b"".join(lines[-(HISTORY_KEEP - 1):]) + line)
        else:
            try:
                f = os.open(HISTORY, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK, 0o644,
                            dir_fd=fd)
            except OSError:  # a link or something else planted there: replaced, not followed
                f = None
            if f is not None and not stat.S_ISREG(os.fstat(f).st_mode):
                os.close(f)
                f = None
            if f is None:
                write_atomic(fd, HISTORY, old + line)
            else:
                try:
                    os.write(f, line)
                finally:
                    os.close(f)
    elif mode == "trainer" and len(argv) == 4:
        data = read_file(argv[3])  # as root, before dropping: the spool is the transfer account's
        j = json_object(data or b"")
        if j is None:
            return
        fd = open_panel(panel)
        new = json.dumps(j).encode()
        if read_file("trainer.json", fd) != new:
            write_atomic(fd, "trainer.json", new)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
