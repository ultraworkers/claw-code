"""Give the CLI terminal stdin and captured stdout, using only Python's stdlib."""
import os
import subprocess
import sys


def run_unix(claw, payload):
    import pty
    master, slave = pty.openpty()
    try:
        with subprocess.Popen([claw], stdin=slave, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE) as child:
            os.close(slave)
            slave = None
            os.write(master, payload)
            try:
                stdout, stderr = child.communicate(timeout=30)
            except subprocess.TimeoutExpired:
                child.kill()
                child.communicate()
                raise
            return child.returncode, stdout, stderr
    finally:
        if slave is not None:
            os.close(slave)
        os.close(master)


def run_windows(claw, payload):
    import ctypes
    import msvcrt
    from ctypes import wintypes

    class KeyEvent(ctypes.Structure):
        _fields_ = [("down", wintypes.BOOL), ("repeat", wintypes.WORD),
                    ("virtual_key", wintypes.WORD), ("scan", wintypes.WORD),
                    ("char", wintypes.WCHAR), ("control", wintypes.DWORD)]

    class Event(ctypes.Union):
        _fields_ = [("key", KeyEvent), ("storage", ctypes.c_byte * 16)]

    class InputRecord(ctypes.Structure):
        _fields_ = [("kind", wintypes.WORD), ("event", Event)]

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.AllocConsole.argtypes = []
    kernel.AllocConsole.restype = wintypes.BOOL
    kernel.FreeConsole.argtypes = []
    kernel.FreeConsole.restype = wintypes.BOOL
    kernel.WriteConsoleInputW.argtypes = [wintypes.HANDLE, ctypes.POINTER(InputRecord),
                                        wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
    kernel.WriteConsoleInputW.restype = wintypes.BOOL
    # This affects only this helper process, never the test runner's console.
    kernel.FreeConsole()
    if not kernel.AllocConsole():
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        with open("CONIN$", "r+b", buffering=0) as console:
            records = []
            for char in payload.decode("utf-8").replace("\r\n", "\n").replace("\n", "\r"):
                for down in (True, False):
                    record = InputRecord()
                    record.kind = 1  # KEY_EVENT
                    record.event.key = KeyEvent(down, 1, 13 if char == "\r" else 0,
                                                0, char, 0)
                    records.append(record)
            inputs = (InputRecord * len(records))(*records)
            written = wintypes.DWORD()
            if not kernel.WriteConsoleInputW(msvcrt.get_osfhandle(console.fileno()),
                                             inputs, len(inputs), ctypes.byref(written)):
                raise ctypes.WinError(ctypes.get_last_error())
            if written.value != len(inputs):
                raise RuntimeError("incomplete console input")
            with subprocess.Popen([claw], stdin=console, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE) as child:
                try:
                    stdout, stderr = child.communicate(timeout=30)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.communicate()
                    raise
                return child.returncode, stdout, stderr
    finally:
        kernel.FreeConsole()


if __name__ == "__main__":
    run = run_windows if os.name == "nt" else run_unix
    code, stdout, stderr = run(sys.argv[1], sys.stdin.buffer.read())
    sys.stdout.buffer.write(stdout)
    sys.stderr.buffer.write(stderr)
    raise SystemExit(code)
