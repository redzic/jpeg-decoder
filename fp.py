# fixed point arithmetic

from sympy import *

x1 = sqrt(2) / 2


# 1 bit decimal point position
dec = 20
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


def addfx(a, b):
    return a + b


# when we multiply, we get a number with higher precision
# so we have to bring it down to the original precision
def mulfx(a, b):
    return (a * b) >> dec


# fixed = d2f(x1)
# fixed2 = d2f(S(33) / 100)

if __name__ == "__main__":
    # fixed = mulfx(d2f(x1), d2f(x2))
    fixed = d2f(x1)

    print(f"  orig: {x1.evalf(PREC)}")
    print(f" fixed: {fixed}")
    print(f"as rat: {f2f(fixed)}")
