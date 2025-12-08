# Monty

A sandboxed Python interpreter written in Rust.

Monty is a **sandboxed Python interpreter** written in Rust. Unlike embedding CPython or using PyO3, Monty implements its own runtime from scratch with these goals:

- **Safety**: Execute untrusted Python code safely without FFI or C dependencies or access to environment, filesystem or network; instead sandbox will call back to host to run foreign/external functions.
- **Performance**: Should be on a par with cpython
- **Simplicity**: Clean, understandable implementation focused on a Python subset
- **Snapshotting and iteration**: Plan is to allow code to be iteratively executed and snapshotted at each function call
