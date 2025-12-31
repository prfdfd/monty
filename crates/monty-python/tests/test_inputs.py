import pytest
from inline_snapshot import snapshot

import monty


def test_single_input():
    m = monty.Monty('x', inputs=['x'])
    assert m.run(inputs={'x': 42}) == snapshot(42)


def test_multiple_inputs():
    m = monty.Monty('x + y + z', inputs=['x', 'y', 'z'])
    assert m.run(inputs={'x': 1, 'y': 2, 'z': 3}) == snapshot(6)


def test_input_used_in_expression():
    m = monty.Monty('x * 2 + y', inputs=['x', 'y'])
    assert m.run(inputs={'x': 5, 'y': 3}) == snapshot(13)


def test_input_string():
    m = monty.Monty('greeting + " " + name', inputs=['greeting', 'name'])
    assert m.run(inputs={'greeting': 'Hello', 'name': 'World'}) == snapshot('Hello World')


def test_input_list():
    m = monty.Monty('data[0] + data[1]', inputs=['data'])
    assert m.run(inputs={'data': [10, 20]}) == snapshot(30)


def test_input_dict():
    m = monty.Monty('config["a"] * config["b"]', inputs=['config'])
    assert m.run(inputs={'config': {'a': 3, 'b': 4}}) == snapshot(12)


def test_missing_input_raises():
    m = monty.Monty('x + y', inputs=['x', 'y'])
    with pytest.raises(KeyError, match="Missing required input: 'y'"):
        m.run(inputs={'x': 1})


def test_all_inputs_missing_raises():
    m = monty.Monty('x', inputs=['x'])
    with pytest.raises(TypeError, match='Missing required inputs'):
        m.run()


def test_no_inputs_declared_but_provided_raises():
    m = monty.Monty('1 + 1')
    with pytest.raises(TypeError, match='No input variables declared but inputs dict was provided'):
        m.run(inputs={'x': 1})
        with pytest.raises(TypeError, match='No input variables declared but inputs dict was provided'):
            m.run(inputs={})


def test_inputs_order_independent():
    m = monty.Monty('a - b', inputs=['a', 'b'])
    # Dict order shouldn't matter
    assert m.run(inputs={'b': 3, 'a': 10}) == snapshot(7)
