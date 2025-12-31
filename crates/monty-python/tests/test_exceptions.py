import pytest
from inline_snapshot import snapshot

import monty


def test_zero_division_error():
    m = monty.Monty('1 / 0')
    with pytest.raises(ZeroDivisionError):
        m.run()


def test_value_error():
    m = monty.Monty("raise ValueError('bad value')")
    with pytest.raises(ValueError, match='bad value'):
        m.run()


def test_type_error():
    m = monty.Monty("'string' + 1")
    with pytest.raises(TypeError):
        m.run()


def test_index_error():
    m = monty.Monty('[1, 2, 3][10]')
    with pytest.raises(IndexError):
        m.run()


def test_key_error():
    m = monty.Monty("{'a': 1}['b']")
    with pytest.raises(KeyError):
        m.run()


def test_attribute_error():
    m = monty.Monty("raise AttributeError('no such attr')")
    with pytest.raises(AttributeError, match='no such attr'):
        m.run()


def test_name_error():
    m = monty.Monty('undefined_variable')
    with pytest.raises(NameError):
        m.run()


def test_assertion_error():
    m = monty.Monty('assert False')
    with pytest.raises(AssertionError):
        m.run()


def test_assertion_error_with_message():
    m = monty.Monty("assert False, 'custom message'")
    with pytest.raises(AssertionError, match='custom message'):
        m.run()


def test_runtime_error():
    m = monty.Monty("raise RuntimeError('runtime error')")
    with pytest.raises(RuntimeError, match='runtime error'):
        m.run()


def test_not_implemented_error():
    m = monty.Monty("raise NotImplementedError('not implemented')")
    with pytest.raises(NotImplementedError, match='not implemented'):
        m.run()


def test_syntax_error_on_init():
    with pytest.raises(SyntaxError):
        monty.Monty('def')


def test_syntax_error_unclosed_paren():
    with pytest.raises(SyntaxError):
        monty.Monty('print(1')


def test_syntax_error_invalid_syntax():
    with pytest.raises(SyntaxError):
        monty.Monty('x = = 1')


def test_raise_caught_exception():
    code = """
try:
    1 / 0
except ZeroDivisionError as e:
    result = 'caught'
result
"""
    m = monty.Monty(code)
    assert m.run() == snapshot('caught')


def test_exception_in_function():
    code = """
def fail():
    raise ValueError('from function')

fail()
"""
    m = monty.Monty(code)
    with pytest.raises(ValueError, match='from function'):
        m.run()


def test_exception_message_preserved():
    m = monty.Monty("raise ValueError('specific message')")
    with pytest.raises(ValueError) as exc_info:
        m.run()
    assert 'specific message' in str(exc_info.value)
