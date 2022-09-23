from sympy import *


x = []
for n in range(8):
    for k in range(8):
        x.append(((n, k), cos(pi * (n + S(1) / 2) * k / 8)))

# print(x)
lut = [[0] * 8] * 8
for x in x:
    y = x[1]
    n = x[0][0]
    k = x[0][1]
    # print(n, k, y)
    print(f"({n},{k}) => {N(y,30)},")
    # lut[n][k] = N(y, 30)

# print(lut)
