from typing import Callable, Literal

from inline_snapshot import snapshot

import monty

PrintCallback = Callable[[Literal['stdout'], str], None]


def make_print_collector() -> tuple[list[str], PrintCallback]:
    """Create a print callback that collects output into a list."""
    output: list[str] = []

    def callback(stream: Literal['stdout'], text: str) -> None:
        assert stream == 'stdout'
        output.append(text)

    return output, callback


def test_print_basic() -> None:
    m = monty.Monty('print("hello")')
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('hello\n')


def test_print_multiple() -> None:
    code = """
print("line 1")
print("line 2")
"""
    m = monty.Monty(code)
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('line 1\nline 2\n')


def test_print_with_values() -> None:
    m = monty.Monty('print(1, 2, 3)')
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('1 2 3\n')


def test_print_with_sep() -> None:
    m = monty.Monty('print(1, 2, 3, sep="-")')
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('1-2-3\n')


def test_print_with_end() -> None:
    m = monty.Monty('print("hello", end="!")')
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('hello!')


def test_print_returns_none() -> None:
    m = monty.Monty('print("test")')
    _, callback = make_print_collector()
    result = m.run(print_callback=callback)
    assert result is None


def test_print_empty() -> None:
    m = monty.Monty('print()')
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('\n')


def test_print_with_limits() -> None:
    """Verify print_callback works together with resource limits."""
    m = monty.Monty('print("with limits")')
    output, callback = make_print_collector()
    limits = monty.ResourceLimits(max_duration_secs=5.0)
    m.run(print_callback=callback, limits=limits)
    assert ''.join(output) == snapshot('with limits\n')


def test_print_with_inputs() -> None:
    """Verify print_callback works together with inputs."""
    m = monty.Monty('print(x)', inputs=['x'])
    output, callback = make_print_collector()
    m.run(inputs={'x': 42}, print_callback=callback)
    assert ''.join(output) == snapshot('42\n')


def test_print_in_loop() -> None:
    code = """
for i in range(3):
    print(i)
"""
    m = monty.Monty(code)
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('0\n1\n2\n')


def test_print_mixed_types() -> None:
    m = monty.Monty('print(1, "hello", True, None)')
    output, callback = make_print_collector()
    m.run(print_callback=callback)
    assert ''.join(output) == snapshot('1 hello True None\n')
