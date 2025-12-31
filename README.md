# Monty

A sandboxed, snapshotable Python interpreter written in Rust.

Monty is a **sandboxed Python interpreter** written in Rust. Unlike embedding CPython or using PyO3,
Monty implements its own runtime from scratc.

The goal is to provide:
* complete safety - no access to the host environment, filesystem or network
* safe access to specific methods on the host
* snapshotting and iterative execution for long running host functions
