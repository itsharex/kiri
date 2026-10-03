"""A real EWMH dock/strut for the disposable Xvfb acceptance desktop only.

This ordinary external X11 window reserves 27 pixels at the top. It never
changes Kiri, a desktop setting, or a user's existing panel configuration.
"""

import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import sys


def serve():
    profile = Path(os.environ.get("KIRI_QA_PROFILE", "/missing")).resolve()
    if not profile.is_dir() or not Path(os.environ["HOME"]).resolve().is_relative_to(profile):
        raise SystemExit("The dock fixture requires the disposable native QA profile")
    x11 = ctypes.CDLL(ctypes.util.find_library("X11"))
    pointer, window = ctypes.c_void_p, ctypes.c_ulong
    signatures = {
        "XOpenDisplay": ([ctypes.c_char_p], pointer),
        "XDefaultRootWindow": ([pointer], window),
        "XCreateSimpleWindow": ([pointer, window, ctypes.c_int, ctypes.c_int,
                                 ctypes.c_uint, ctypes.c_uint, ctypes.c_uint,
                                 ctypes.c_ulong, ctypes.c_ulong], window),
        "XInternAtom": ([pointer, ctypes.c_char_p, ctypes.c_int], ctypes.c_ulong),
        "XChangeProperty": ([pointer, window, ctypes.c_ulong, ctypes.c_ulong,
                             ctypes.c_int, ctypes.c_int, pointer, ctypes.c_int], ctypes.c_int),
        "XStoreName": ([pointer, window, ctypes.c_char_p], ctypes.c_int),
        "XMapWindow": ([pointer, window], ctypes.c_int),
        "XSync": ([pointer, ctypes.c_int], ctypes.c_int),
        "XDestroyWindow": ([pointer, window], ctypes.c_int),
        "XCloseDisplay": ([pointer], ctypes.c_int),
    }
    for name, (arguments, result) in signatures.items():
        getattr(x11, name).argtypes = arguments
        getattr(x11, name).restype = result
    display = x11.XOpenDisplay(None)
    if not display:
        raise RuntimeError("Cannot open the isolated X11 display")
    dock = None
    try:
        dock = x11.XCreateSimpleWindow(display, x11.XDefaultRootWindow(display),
                                      0, 0, 1280, 27, 0, 0, 0x314659)

        def atom(name):
            return x11.XInternAtom(display, name.encode(), False)

        def property_(name, type_, values):
            # Xlib's format-32 input uses native longs even on LP64 machines.
            data = (ctypes.c_ulong * len(values))(*values)
            x11.XChangeProperty(display, dock, atom(name), atom(type_),
                                32, 0, data, len(values))

        x11.XStoreName(display, dock, b"Kiri QA panel with a 27px top strut")
        property_("_NET_WM_WINDOW_TYPE", "ATOM", [atom("_NET_WM_WINDOW_TYPE_DOCK")])
        property_("_NET_WM_STRUT", "CARDINAL", [0, 0, 27, 0])
        property_("_NET_WM_STRUT_PARTIAL", "CARDINAL", [0, 0, 27, 0, 0, 0, 0, 0, 0, 1279, 0, 0])
        x11.XMapWindow(display, dock)
        x11.XSync(display, False)
        print(json.dumps({"window": dock, "height": 27}), flush=True)
        for line in sys.stdin:
            if line.strip() == "quit":
                break
    finally:
        if dock:
            x11.XDestroyWindow(display, dock)
        x11.XCloseDisplay(display)


if __name__ == "__main__":
    serve()
