# Test fixture for exec() template detection
# This file simulates CPython patterns where exec() generates functions dynamically.
# INTENTIONALLY uses exec() - this is a test fixture for loctree's dynamic code detection.

# Pattern 1: Simple exec with %s format
# These exported functions (get_foo, set_foo) should NOT be flagged as dead code
def _make_accessor(name):
    """Generate getter/setter methods dynamically using exec()."""
    code = """
def get%s(self):
    return self._%s

def set%s(self, value):
    self._%s = value
""" % (name, name, name, name)
    exec(code, globals())  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

# Generate accessors at module load time
_make_accessor('foo')
_make_accessor('bar')

# Pattern 2: eval with {name} format (f-string style in template)
def _make_validator(field_name):
    """Generate validator function dynamically using compile()."""
    code = f"""
def validate_{field_name}(value):
    if not isinstance(value, str):
        raise ValueError("Expected string for {field_name}")
    return value
"""
    compiled = compile(code, '<validator>', 'exec')
    exec(compiled, globals())  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

_make_validator('username')
_make_validator('email')

# Pattern 3: Class generation with exec
def _make_model(model_name):
    """Generate model class dynamically."""
    code = """
class %s:
    def __init__(self):
        pass
""" % model_name
    exec(code, globals())  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

_make_model('User')
_make_model('Product')

# Regular exports (should be detected as dead if not used)
def regular_function():
    """This is a normal function that can be flagged as dead if unused."""
    pass

class RegularClass:
    """This is a normal class that can be flagged as dead if unused."""
    pass
