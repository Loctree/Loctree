"""
Test fixture for Python exec() template pattern detection.

This file demonstrates metaprogramming patterns that generate functions
dynamically using exec(), eval(), and compile(). These should be detected
as dynamic generation and marked appropriately.
"""

# Pattern 1: exec() with %s template string
# CPython uses this pattern in Lib/opcode.py for generating constants
template_percent = '''
def get%s(self):
    return self._obj.%s
'''

# Generate getFoo, setFoo methods
for name in ['Foo', 'Bar', 'Baz']:
    exec(template_percent % ((name,) * 2))  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

# Pattern 2: exec() with f-string template
template_fstring = '''
def process_{name}(value):
    return value.{name}()
'''

for name in ['upper', 'lower', 'title']:
    exec(template_fstring.format(name=name))  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

# Pattern 3: exec() with format() template
template_format = '''
class {classname}Handler:
    def handle(self):
        pass
'''

for cls in ['Event', 'Command', 'Query']:
    exec(template_format.format(classname=cls))  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

# Pattern 4: compile() with template strings
code_template = '''
result_{i} = lambda x: x * {i}
'''

for i in range(5):
    code = compile(code_template.format(i=i), '<string>', 'exec')
    exec(code)  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

# Pattern 5: eval() with template (less common, but should detect)
eval_template = 'CONSTANT_{} = {}'

globals_dict = {}
for i, val in enumerate(['alpha', 'beta', 'gamma']):
    eval(eval_template.format(val.upper(), i), globals_dict)  # nosemgrep - TEST FIXTURE for loctree dynamic code detection

# Not a template pattern - should NOT be flagged
def regular_function():
    """This is a regular function, not dynamically generated."""
    return 42

# exec() without template - should NOT be flagged
exec("x = 1")  # nosemgrep - TEST FIXTURE (non-template pattern)

# Regular class - should NOT be flagged
class RegularClass:
    def method(self):
        return "static"
