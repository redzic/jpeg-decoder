#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

const uint8_t JPEG_MARKER[2] = {0xff, 0xd8};

// umm... does this actually work on big/little endian?
static int is_jpeg_marker(uint8_t header[2]) {
  return (*(uint16_t *)header) == (*(uint16_t *)JPEG_MARKER);
}

struct ApplicationHeader {
  char identifier[5];

  uint8_t version_major;
  uint8_t version_minor;

  uint8_t units;

  uint16_t density_x;
  uint16_t density_y;

  uint8_t thumbnail_x;
  uint8_t thumbnail_y;
};

int main() {
  uint8_t *buf = malloc(32);

  //   FILE *f = fopen("./main.c", "rb");
  FILE *f = fopen("./out.jpg", "rb");
  if (!f) {
    fprintf(stderr, "jpegdec: file does not exist\n");
    exit(1);
  }

  // python does loop with >H, which apparently means
  // read a big-endian unsigned short (u16)

  size_t n = fread(buf, sizeof(uint8_t), 32, f);
  assert(n);

  printf("Is file JPEG? %d\n", is_jpeg_marker(buf));

  for (size_t i = 0; i < 2; i++) {
    printf("0x%x ", buf[i]);
  }

  puts("");
}