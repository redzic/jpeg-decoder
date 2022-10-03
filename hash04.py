# basic idea of hashemian 2004 paper

# symbol list, in sorted order
s = [x + 1 for x in range(18)]

# 10 01 10 00 00 01 10 110 111000
# 3 2 3 1 1 2 3 4 5

bitcount = []

for x in [(2, 3), (3, 1), (6, 3), (7, 9), (8, 2)]:
    # (code length, count)
    for y in range(x[1]):
        bitcount.append(x[0])


c = [
    # 1
    0b00,
    # 2
    0b01,
    # 3
    0b10,
    # 4
    0b110,
    # 5
    0b111000,
    0b111001,
    0b111010,
    0b1110110,
    0b1110111,
    0b1111000,
    0b1111001,
    0b1111010,
    0b1111011,
    0b1111100,
    0b1111101,
    0b1111110,
    0b11111110,
    0b11111111,
]

# TODO derive condensed huffman table based on codewords
# and write random tests to verify decoding

# we need a map of what to actually subtract by

# for jpeg we probably need to "augment" to 16 bits

# condensed huffman table representation
# [augmented code C, code length L, symbol position N]
cht = [
    [0xC0, 3, 4],
    [0xE0, 6, 5],
    [0xEC, 7, 8],
    [0xFE, 8, 17],
]


# min code length
l0 = 2
# max code length
lm = 8

# Get bitstream from symbols
def code_symbols(symbols):
    bitbuf = 0
    nbits = 0
    # 3 2 3 1 1 2 3 4 5 18 3 13 18 2 2 7 7 7
    for symbol in symbols:
        bits = bitcount[symbol - 1]
        # shift current buffer
        bitbuf <<= bits
        # append code
        bitbuf |= c[symbol - 1]
        nbits += bits

    return (bitbuf, nbits)


class BitStream:
    def __init__(self, bits, bitstream):
        self.bits = bits
        self.buflen = bits
        self.bitstream = bitstream

    def GetCode(self):
        W = self.PeekBits(lm)

        # first index changes when searching for match
        if W <= cht[0][0]:
            # get top L_0 bits
            W >>= lm - l0

            bs.ConsumeBits(l0)

            return s[W]
        else:
            # find first codeword greater than W
            j = None
            for i in range(1, len(cht)):
                if cht[i][0] > W:
                    j = i - 1
                    break
            if j is None:
                j = len(cht) - 1

            # L_i = j-1
            # codeword length
            L = cht[j][1]
            W >>= lm - L

            bs.ConsumeBits(L)

            base = cht[j][0] >> (lm - cht[j][1])
            offset = cht[j][2] - 1

            idx = W - base + offset
            assert idx >= 0
            return s[idx]

    # advance bitstream without returning the bits
    def ConsumeBits(self, nbits):

        # clear out top nbits before shifting
        # (not necessary with fixed size integers)
        self.bitstream &= ((1 << self.buflen) - 1) >> nbits
        self.bitstream <<= nbits

        self.bits -= nbits

        # assert self.bits >= nbits

        # if not (self.bits >= nbits):
        # print("(assertion failed)")

        return

    def PeekBits(self, nbits):
        # assert self.bits >= nbits

        return self.bitstream >> (self.buflen - nbits)

    def GetBits(self, nbits):
        bits = self.PeekBits(nbits)
        self.ConsumeBits(nbits)

        return bits


data = [x for x in range(1, 19)][::-1]


bitstream, bits = code_symbols(data)

bs = BitStream(bits, bitstream)


decoded_buffer = []
for _ in range(len(data)):
    decoded_buffer.append(bs.GetCode())

print(f" source: {data}")
print(f"decoded: {decoded_buffer}")

assert data == decoded_buffer
