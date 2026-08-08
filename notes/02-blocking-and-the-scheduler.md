---
title: Blocking, What a Thread Does While It Waits
date: 2026-07-31
description: incoming().next() calls accept in the kernel. No client means the scheduler takes the thread off the CPU at zero cost; a connection arriving makes it runnable again. This iterator never returns None.
draft: true
---

# Blocking: what a thread does while it waits

`listener.incoming()` returns an iterator object that `.next()` gets called on. Each `.next()` call runs `accept` in the kernel. If a client is connecting, `.next()` returns `Some(Ok(stream))`, or `Some(Err(e))` if the connection failed.

If no client is connecting, the thread blocks inside the `accept` call: the kernel scheduler removes the thread from the CPU and stops giving it turns, so waiting costs no CPU at all. The thread's stack stays in RAM, frozen mid-call. When a connection arrives, the kernel marks the thread runnable, the scheduler puts it back in rotation for the CPU to run it, and then `accept` returns.

This `.next()` never returns `None`. A listening socket is never finished, so the loop never ends. The array iterator answered "no more" with `None`; this one answers with waiting.
