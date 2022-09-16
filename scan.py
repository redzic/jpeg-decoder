# fmt: off
zigzag = [
	 0,  1,  8, 16,  9,  2,  3, 10,
	17, 24, 32, 25, 18, 11,  4,  5,
	12, 19,	26, 33, 40, 48, 41, 34,
	27, 20, 13,  6,  7, 14, 21, 28,
	35, 42, 49, 56, 57, 50, 43, 36,
	29, 22, 15, 23, 30, 37, 44, 51,
	58, 59, 52, 45, 38, 31, 39, 46,
	53, 60, 61, 54, 47, 55, 62, 63
]
# fmt: on

x = []

for i in range(64):
    x.append((i, zigzag[i]))

y = sorted(x, key=lambda z: z[1])
z = list(map(lambda w: w[0], y))

# print(y)
print(z)

for i in range(8):
    for j in range(8):
        xy = 8 * i + j
        print(f"{z[xy]:>2},", end=" ")
    print()
