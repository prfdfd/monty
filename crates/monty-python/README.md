# monty

Python bindings for the Monty sandboxed Python interpreter.

## Installation

```bash
pip install monty-python
```

## Usage

### Basic Expression Evaluation

```python
import monty

# Simple code with no inputs
m = monty.Monty('1 + 2')
assert m.run() == 3
```

### Using Input Variables

```python
import monty

# Create with code that uses input variables
m = monty.Monty('x * y', inputs=['x', 'y'])

# Run multiple times with different inputs
assert m.run(inputs={'x': 2, 'y': 3}) == 6
assert m.run(inputs={'x': 10, 'y': 5}) == 50
```

### Resource Limits

```python
import monty

m = monty.Monty('x + y', inputs=['x', 'y'])

# With resource limits
limits = monty.ResourceLimits(max_duration_secs=1.0)
result = m.run(inputs={'x': 1, 'y': 2}, limits=limits)
assert result == 3
```

### External Functions

```python
import monty

# Code that calls an external function
m = monty.Monty('double(x)', inputs=['x'], external_functions=['double'])

# Provide the external function implementation at runtime
result = m.run(inputs={'x': 5}, external_functions={'double': lambda x: x * 2})
assert result == 10
```
