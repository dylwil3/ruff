class Foo1:
    """abc"""


class Foo2:
    """abc"""

    a = 2
    "str"  # Str (no raise)
    f"{int}"  # JoinedStr (no raise)
    1j  # Number (complex)
    1  # Number (int)
    1.0  # Number (float)
    b"foo"  # Binary
    True  # NameConstant (True)
    False  # NameConstant (False)
    None  # NameConstant (None)
    [1, 2]  # list
    {1, 2}  # set
    {"foo": "bar"}  # dict


class Foo3:
    123
    a = 2
    "str"
    1


def foo1():
    """my docstring"""


def foo2():
    """my docstring"""
    a = 2
    "str"  # Str (no raise)
    f"{int}"  # JoinedStr (no raise)
    1j  # Number (complex)
    1  # Number (int)
    1.0  # Number (float)
    b"foo"  # Binary
    True  # NameConstant (True)
    False  # NameConstant (False)
    None  # NameConstant (None)
    [1, 2]  # list
    {1, 2}  # set
    {"foo": "bar"}  # dict


def foo3():
    123
    a = 2
    "str"
    3


def foo4():
    ...


def foo5():
    foo.bar  # Attribute (raise)
    object().__class__  # Attribute (raise)
    "foo" + "bar"  # BinOp (raise)

# See https://github.com/astral-sh/ruff/issues/14131
def foo6(): ...

# Ok
foo6()
[foo6() for _ in range(10)]
{foo6() for _ in range(10)}
{"a":foo6() for _ in range(10)}
# Raise
[1, 2, foo6()]
print("foo"),
[x for x in [1,2,3]]
[x for x in foo6()]
[x for x in range(10)]
foo6() + foo6()
(foo6() for _ in range(10))

