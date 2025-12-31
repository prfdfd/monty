from typing import Any

import pytest
from inline_snapshot import snapshot

import monty


def test_external_function_no_args():
    m = monty.Monty('noop()', external_functions=['noop'])

    def noop(*args: Any, **kwargs: Any) -> str:
        assert args == snapshot(())
        assert kwargs == snapshot({})
        return 'called'

    assert m.run(external_functions={'noop': noop}) == snapshot('called')


def test_external_function_positional_args():
    m = monty.Monty('func(1, 2, 3)', external_functions=['func'])

    def func(*args: Any, **kwargs: Any) -> str:
        assert args == snapshot((1, 2, 3))
        assert kwargs == snapshot({})
        return 'ok'

    assert m.run(external_functions={'func': func}) == snapshot('ok')


def test_external_function_kwargs_only():
    m = monty.Monty('func(a=1, b="two")', external_functions=['func'])

    def func(*args: Any, **kwargs: Any) -> str:
        assert args == snapshot(())
        assert kwargs == snapshot({'a': 1, 'b': 'two'})
        return 'ok'

    assert m.run(external_functions={'func': func}) == snapshot('ok')


def test_external_function_mixed_args_kwargs():
    m = monty.Monty('func(1, 2, x="hello", y=True)', external_functions=['func'])

    def func(*args: Any, **kwargs: Any) -> str:
        assert args == snapshot((1, 2))
        assert kwargs == snapshot({'x': 'hello', 'y': True})
        return 'ok'

    assert m.run(external_functions={'func': func}) == snapshot('ok')


def test_external_function_complex_types():
    m = monty.Monty('func([1, 2], {"key": "value"})', external_functions=['func'])

    def func(*args: Any, **kwargs: Any) -> str:
        assert args == snapshot(([1, 2], {'key': 'value'}))
        assert kwargs == snapshot({})
        return 'ok'

    assert m.run(external_functions={'func': func}) == snapshot('ok')


def test_external_function_returns_none():
    m = monty.Monty('do_nothing()', external_functions=['do_nothing'])

    def do_nothing(*args: Any, **kwargs: Any) -> None:
        assert args == snapshot(())
        assert kwargs == snapshot({})

    assert m.run(external_functions={'do_nothing': do_nothing}) is None


def test_external_function_returns_complex_type():
    m = monty.Monty('get_data()', external_functions=['get_data'])

    def get_data(*args: Any, **kwargs: Any) -> dict[str, Any]:
        return {'a': [1, 2, 3], 'b': {'nested': True}}

    result = m.run(external_functions={'get_data': get_data})
    assert result == snapshot({'a': [1, 2, 3], 'b': {'nested': True}})


def test_multiple_external_functions():
    m = monty.Monty('add(1, 2) + mul(3, 4)', external_functions=['add', 'mul'])

    def add(*args: Any, **kwargs: Any) -> int:
        assert args == snapshot((1, 2))
        assert kwargs == snapshot({})
        return args[0] + args[1]

    def mul(*args: Any, **kwargs: Any) -> int:
        assert args == snapshot((3, 4))
        assert kwargs == snapshot({})
        return args[0] * args[1]

    result = m.run(external_functions={'add': add, 'mul': mul})
    assert result == snapshot(15)  # 3 + 12


def test_external_function_called_multiple_times():
    m = monty.Monty('counter() + counter() + counter()', external_functions=['counter'])

    call_count = 0

    def counter(*args: Any, **kwargs: Any) -> int:
        nonlocal call_count
        assert args == snapshot(())
        assert kwargs == snapshot({})
        call_count += 1
        return call_count

    result = m.run(external_functions={'counter': counter})
    assert result == snapshot(6)  # 1 + 2 + 3
    assert call_count == snapshot(3)


def test_external_function_with_input():
    m = monty.Monty('process(x)', inputs=['x'], external_functions=['process'])

    def process(*args: Any, **kwargs: Any) -> int:
        assert args == snapshot((5,))
        assert kwargs == snapshot({})
        return args[0] * 10

    assert m.run(inputs={'x': 5}, external_functions={'process': process}) == snapshot(50)


def test_external_function_not_provided_raises():
    m = monty.Monty('missing()', external_functions=['missing'])

    with pytest.raises(RuntimeError, match='no external_functions provided'):
        m.run()


def test_undeclared_function_raises_name_error():
    m = monty.Monty('unknown_func()')

    with pytest.raises(NameError, match="name 'unknown_func' is not defined"):
        m.run()


# TODO: Raising exceptions from external function callbacks is not yet implemented in Monty.
# When called, it panics rather than propagating the exception.
# def test_external_function_raises_exception():
#     m = monty.Monty('fail()', external_functions=['fail'])
#
#     def fail(*args: Any, **kwargs: Any) -> None:
#         raise ValueError('intentional error')
#
#     with pytest.raises(ValueError, match='intentional error'):
#         m.run(external_functions={'fail': fail})
#
#
# def test_external_function_wrong_name_raises():
#     m = monty.Monty('foo()', external_functions=['foo'])
#
#     def bar(*args: Any, **kwargs: Any) -> int:
#         return 1
#
#     # This should raise KeyError but currently panics
#     with pytest.raises(KeyError, match="'foo' not found"):
#         m.run(external_functions={'bar': bar})
