# arr = [0, 2, 3, 2]
arr = [0, 2, 2, 3, 1, 1, 1]

code = 0
bits = 0

codes = []


# for x in arr:
for x in arr:
    code <<= 1
    bits += 1

    for y in range(x):
        codes.append(bin(code)[2:].zfill(bits))
        code += 1

for x in codes:
    print(x)
