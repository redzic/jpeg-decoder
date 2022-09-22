#include <cmath>
#include <cstring>
#include <iostream>

// (x, X) = (input, output)
template <auto N>
void dct_ii(double *__restrict__ in, double *__restrict__ out) {
  for (size_t k = 0; k < N; k++) {
    double sum = 0.;
    double s = (k == 0) ? sqrt(.5) : 1.;
    for (size_t n = 0; n < N; n++) {
      sum += s * in[n] * cos(M_PI * (n + .5) * k / N);
    }
    out[k] = sum * sqrt(2. / N);
  }
}

void transpose_8x8(double *__restrict in, double *__restrict out) {
  for (size_t i = 0; i < 8; i++) {
    for (size_t j = 0; j < 8; j++) {
      out[j * 8 + i] = in[i * 8 + j];
    }
  }
}

// x = input
// X = output
void fdct_8x8(double *__restrict__ in, double *__restrict__ out) {
  // 1D transform on each row
  for (size_t i = 0; i < 8; i++) {
    dct_ii<8>(&in[8 * i], &out[8 * i]);
  }

  double transposed[64];

  // transpose, do another
  transpose_8x8(out, transposed);

  // 1D transform on each column
  for (size_t i = 0; i < 8; i++) {
    dct_ii<8>(&transposed[8 * i], &out[8 * i]);
  }
}

template <auto N>
void idct(const double *__restrict__ X, double *__restrict__ x) {
  for (size_t n = 0; n < N; ++n) {
    double sum = 0.;
    for (size_t k = 0; k < N; ++k) {
      double s = (k == 0) ? sqrt(.5) : 1.;
      sum += s * X[k] * cos(M_PI * (n + .5) * k / N);
    }
    x[n] = sum * sqrt(2. / N);
  }
}

void idct_8x8(double *__restrict__ in, double *__restrict__ out) {
  // 1D transform on each row
  for (size_t i = 0; i < 8; i++) {
    idct<8>(&in[8 * i], &out[8 * i]);
  }

  double transposed[64];

  // transpose, do another
  transpose_8x8(out, transposed);

  // 1D transform on each column
  for (size_t i = 0; i < 8; i++) {
    idct<8>(&transposed[8 * i], &out[8 * i]);
  }
}

template <auto N>
void idct_2d(const double *__restrict__ X, double *__restrict__ x) {
  for (size_t n = 0; n < N; ++n) {
    double sum = 0.;
    for (size_t k = 0; k < N; ++k) {
      double s = (k == 0) ? sqrt(.5) : 1.;
      sum += s * X[k] * cos(M_PI * (n + .5) * k / N);
    }
    x[n] = sum * sqrt(2. / N);
  }
}

template <auto N> void disp_NxN_matrix(const double *__restrict m) {
  for (size_t i = 0; i < N; i++) {
    for (size_t j = 0; j < N; j++) {
      std::cout << m[i * N + j] << " ";
    }
    std::puts("");
  }
}

auto main() -> int {
  double block[64];

  memset(block, 0, 64 * sizeof(double));
  block[0] = 3.0;
  block[8] = 69.0;

  // disp_NxN_matrix<8>(block);

  double dct_coeffs[64];

  fdct_8x8(block, dct_coeffs);

  // disp_NxN_matrix<8>(dct_coeffs);

  double block2[64];

  idct_8x8(dct_coeffs, block2);

  disp_NxN_matrix<8>(block2);

  return 0;
}
