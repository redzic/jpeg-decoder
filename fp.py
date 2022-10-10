# fixed point arithmetic

from sympy import *

x1 = sqrt(2) / 2


# 1 bit decimal point position
dec = 12
# total bits in number
bits = 32
# so that leaves us with 31 bits of the integer part

# display precision
PREC = 15


# fixed->rational conversion
def f2r(x):
    # integer part
    integer = x >> dec
    # fractional part
    frac = x & ((1 << dec) - 1)

    y = integer + frac / S(1 << dec)
    return y


# fixed->float conversion
def f2f(x):
    return f2r(x).evalf(PREC)


# decimal->fixed conversion
def d2f(x):
    # integer part
    return int(round(x * S(1 << dec)))


def x2f(x):
    return d2f(nsimplify(x))


# when we multiply, we get a number with higher precision
# so we have to bring it down to the original precision
def mul(a, b):
    # return (a * b) >> dec
    return (a * b) / (2**dec)


# fixed = d2f(x1)
# fixed2 = d2f(S(33) / 100)

if __name__ == "__main__":
    print(d2f(128))

    y = Symbol("y")
    cb = Symbol("cb")
    cr = Symbol("cr")

    # r = x2f(1.402) * (cr - x2f(128.0)) + y
    r = mul(x2f(1.402), cr - x2f(128.0)) + y
    g = mul(x2f(-0.71414), cr - x2f(128.0)) + mul(x2f(-0.34414), cb - x2f(128.0)) + y
    b = mul(x2f(1.772), cb - x2f(128.0)) + y

    print(simplify(r))
    print(simplify(g))
    print(simplify(b))

    # fixed = mulfx(d2f(x1), d2f(x2))
    # fixed = d2f(x1)

    # print(f"  orig: {x1.evalf(PREC)}")
    # print(f" fixed: {fixed}")
    # print(f"as rat: {f2f(fixed)}")
