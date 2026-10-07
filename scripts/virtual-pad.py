#!/usr/bin/env python3
"""Testing aid for Linux: creates a virtual controller through /dev/uinput and
plays a script of button presses, so the desktop build's gamepad support can be
checked without a physical pad. The game reads it like any other controller.

Every program on this machine that reads controllers sees the device while it
exists, so keep scripts short. Usage, one step per argument:

    python3 scripts/virtual-pad.py "wait 3" "tap start" "hold right 1.5" \
        "tap a" "stick 0.8 0" "wait 1" "stick 0 0" "tap rb"

Steps: wait SECONDS, tap BUTTON [SECONDS], hold BUTTON SECONDS,
down BUTTON, up BUTTON, stick X Y (each -1..1, y down).
Buttons: a b x y lb rb lt rt view start, and up down left right for the D-pad.
"""
import fcntl
import os
import struct
import sys
import time

EV_SYN, EV_KEY, EV_ABS = 0, 1, 3
# Kernel codes: BTN_SOUTH, BTN_EAST, BTN_WEST (0x134), and BTN_NORTH (0x133).
BUTTONS = {
    "a": 0x130, "b": 0x131, "x": 0x134, "y": 0x133,
    "lb": 0x136, "rb": 0x137, "lt": 0x138, "rt": 0x139,
    "view": 0x13A, "start": 0x13B,
}
HAT = {"left": (0x10, -1), "right": (0x10, 1), "up": (0x11, -1), "down": (0x11, 1)}
ABS_X, ABS_Y, ABS_RX, ABS_RY = 0x00, 0x01, 0x03, 0x04


def ioc(direction, number, size):
    return (direction << 30) | (size << 16) | (ord("U") << 8) | number


UI_SET_EVBIT = ioc(1, 100, 4)
UI_SET_KEYBIT = ioc(1, 101, 4)
UI_SET_ABSBIT = ioc(1, 103, 4)
UI_DEV_SETUP = ioc(1, 3, 92)
UI_ABS_SETUP = ioc(1, 4, 28)
UI_DEV_CREATE = ioc(0, 1, 0)
UI_DEV_DESTROY = ioc(0, 2, 0)


class Pad:
    def __init__(self):
        self.fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_KEY)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_ABS)
        for code in BUTTONS.values():
            fcntl.ioctl(self.fd, UI_SET_KEYBIT, code)
        axes = [(code, -32768, 32767) for code in (ABS_X, ABS_Y, ABS_RX, ABS_RY)]
        axes += [(0x10, -1, 1), (0x11, -1, 1)]
        for code, low, high in axes:
            fcntl.ioctl(self.fd, UI_SET_ABSBIT, code)
            fcntl.ioctl(self.fd, UI_ABS_SETUP, struct.pack("HH6i", code, 0, 0, low, high, 0, 0, 0))
        # BUS_USB, the pid.codes test vendor, and a product number of our own.
        name = b"Cinderwake virtual test pad"
        fcntl.ioctl(self.fd, UI_DEV_SETUP, struct.pack("4H80sI", 3, 0x1209, 0xC1D0, 1, name, 0))
        fcntl.ioctl(self.fd, UI_DEV_CREATE)

    def emit(self, kind, code, value):
        os.write(self.fd, struct.pack("qqHHi", 0, 0, kind, code, value))
        os.write(self.fd, struct.pack("qqHHi", 0, 0, EV_SYN, 0, 0))

    def button(self, name, down):
        if name in HAT:
            code, value = HAT[name]
            self.emit(EV_ABS, code, value if down else 0)
        else:
            self.emit(EV_KEY, BUTTONS[name], int(down))

    def stick(self, x, y):
        self.emit(EV_ABS, ABS_X, int(max(-1, min(1, x)) * 32767))
        self.emit(EV_ABS, ABS_Y, int(max(-1, min(1, y)) * 32767))

    def close(self):
        fcntl.ioctl(self.fd, UI_DEV_DESTROY)
        os.close(self.fd)


def main(steps):
    pad = Pad()
    print("virtual pad ready", flush=True)
    try:
        for step in steps:
            words = step.split()
            verb, args = words[0], words[1:]
            print(f"{time.monotonic():.2f} {step}", flush=True)
            if verb == "wait":
                time.sleep(float(args[0]))
            elif verb in ("tap", "hold"):
                pad.button(args[0], True)
                time.sleep(float(args[1]) if len(args) > 1 else 0.12)
                pad.button(args[0], False)
                time.sleep(0.12)
            elif verb == "down":
                pad.button(args[0], True)
            elif verb == "up":
                pad.button(args[0], False)
            elif verb == "stick":
                pad.stick(float(args[0]), float(args[1]))
            else:
                raise SystemExit(f"unknown step: {step}")
    finally:
        pad.close()


if __name__ == "__main__":
    main(sys.argv[1:])
