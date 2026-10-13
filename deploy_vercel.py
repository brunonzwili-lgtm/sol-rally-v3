#!/usr/bin/env python3
"""Upload the staged Sol Rally static site to Vercel production.

Run detached (double-fork + setsid) because the ~390MB upload outlives
any shell timeout, mirroring what serve.py does for the dev server.
"""
import os

STAGE = "/Users/h/Documents/stk-code-wasm/deploy"
LOG = "/tmp/vercel_deploy.log"

if os.fork() > 0:
    os._exit(0)

os.setsid()

if os.fork() > 0:
    os._exit(0)

os.chdir(STAGE)
log = os.open(LOG, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)
os.dup2(log, 1)
os.dup2(log, 2)
devnull = os.open(os.devnull, os.O_RDONLY)
os.dup2(devnull, 0)

os.execvp("vercel", ["vercel", "deploy", "--prod", "--yes", "--archive=tgz"])