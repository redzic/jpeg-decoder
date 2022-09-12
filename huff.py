arr = [0, 2, 3, 2]

code = 0
bits = 0

codes = []


# then the codes are printed in a really confusing way
for x in arr:
    code <<= 1
    bits += 1

    for y in range(x):
        codes.append(bin(code)[2:].zfill(bits))
        code += 1

for x in codes:
    print(x)
