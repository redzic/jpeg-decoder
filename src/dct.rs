use std::arch::x86_64::*;

#[inline(always)]
unsafe fn cast<const N: usize, T>(x: &[T]) -> &[T; N] {
    &*(x as *const [T] as *const [T; N])
}

#[inline(always)]
unsafe fn cast_mut<const N: usize, T>(x: &mut [T]) -> &mut [T; N] {
    &mut *(x as *mut [T] as *mut [T; N])
}

// fn transpose8x8(inm: &[f32; 64], outm: &mut [f32; 64]) {
//     for i in 0..8 {
//         for j in 0..8 {
//             outm[j * 8 + i] = inm[i * 8 + j];
//         }
//     }
// }

fn transpose8x8(input: &[f32; 64], into: &mut [f32; 64]) {
    unsafe {
        let input = [
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(0)),
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(1)),
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(2)),
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(3)),
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(4)),
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(5)),
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(6)),
            _mm256_loadu_si256(input.as_ptr().cast::<__m256i>().add(7)),
        ];

        let stage1 = (
            _mm256_unpacklo_epi32(input[0], input[1]),
            _mm256_unpackhi_epi32(input[0], input[1]),
            _mm256_unpacklo_epi32(input[2], input[3]),
            _mm256_unpackhi_epi32(input[2], input[3]),
            _mm256_unpacklo_epi32(input[4], input[5]),
            _mm256_unpackhi_epi32(input[4], input[5]),
            _mm256_unpacklo_epi32(input[6], input[7]),
            _mm256_unpackhi_epi32(input[6], input[7]),
        );

        let stage2 = (
            _mm256_unpacklo_epi64(stage1.0, stage1.2),
            _mm256_unpackhi_epi64(stage1.0, stage1.2),
            _mm256_unpacklo_epi64(stage1.1, stage1.3),
            _mm256_unpackhi_epi64(stage1.1, stage1.3),
            _mm256_unpacklo_epi64(stage1.4, stage1.6),
            _mm256_unpackhi_epi64(stage1.4, stage1.6),
            _mm256_unpacklo_epi64(stage1.5, stage1.7),
            _mm256_unpackhi_epi64(stage1.5, stage1.7),
        );

        #[allow(clippy::identity_op)]
        const LO: i32 = (2 << 4) | 0;
        const HI: i32 = (3 << 4) | 1;
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(0),
            _mm256_permute2x128_si256(stage2.0, stage2.4, LO),
        );
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(1),
            _mm256_permute2x128_si256(stage2.1, stage2.5, LO),
        );
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(2),
            _mm256_permute2x128_si256(stage2.2, stage2.6, LO),
        );
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(3),
            _mm256_permute2x128_si256(stage2.3, stage2.7, LO),
        );
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(4),
            _mm256_permute2x128_si256(stage2.0, stage2.4, HI),
        );
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(5),
            _mm256_permute2x128_si256(stage2.1, stage2.5, HI),
        );
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(6),
            _mm256_permute2x128_si256(stage2.2, stage2.6, HI),
        );
        _mm256_storeu_si256(
            into.as_mut_ptr().cast::<__m256i>().add(7),
            _mm256_permute2x128_si256(stage2.3, stage2.7, HI),
        );
    }
}

const COS_TABLE: [f32; 64] = [
    1.00000000000000000000000000000,
    0.980785280403230449126182236134,
    0.923879532511286756128183189397,
    0.831469612302545237078788377618,
    0.707106781186547524400844362105,
    0.555570233019602224742830813949,
    0.382683432365089771728459984030,
    0.195090322016128267848284868477,
    1.00000000000000000000000000000,
    0.831469612302545237078788377618,
    0.382683432365089771728459984030,
    -0.195090322016128267848284868477,
    -0.707106781186547524400844362105,
    -0.980785280403230449126182236134,
    -0.923879532511286756128183189397,
    -0.555570233019602224742830813949,
    1.00000000000000000000000000000,
    0.555570233019602224742830813949,
    -0.382683432365089771728459984030,
    -0.980785280403230449126182236134,
    -0.707106781186547524400844362105,
    0.195090322016128267848284868477,
    0.923879532511286756128183189397,
    0.831469612302545237078788377618,
    1.00000000000000000000000000000,
    0.195090322016128267848284868477,
    -0.923879532511286756128183189397,
    -0.555570233019602224742830813949,
    0.707106781186547524400844362105,
    0.831469612302545237078788377618,
    -0.382683432365089771728459984030,
    -0.980785280403230449126182236134,
    1.00000000000000000000000000000,
    -0.195090322016128267848284868477,
    -0.923879532511286756128183189397,
    0.555570233019602224742830813949,
    0.707106781186547524400844362105,
    -0.831469612302545237078788377618,
    -0.382683432365089771728459984030,
    0.980785280403230449126182236134,
    1.00000000000000000000000000000,
    -0.555570233019602224742830813949,
    -0.382683432365089771728459984030,
    0.980785280403230449126182236134,
    -0.707106781186547524400844362105,
    -0.195090322016128267848284868477,
    0.923879532511286756128183189397,
    -0.831469612302545237078788377618,
    1.00000000000000000000000000000,
    -0.831469612302545237078788377618,
    0.382683432365089771728459984030,
    0.195090322016128267848284868477,
    -0.707106781186547524400844362105,
    0.980785280403230449126182236134,
    -0.923879532511286756128183189397,
    0.555570233019602224742830813949,
    1.00000000000000000000000000000,
    -0.980785280403230449126182236134,
    0.923879532511286756128183189397,
    -0.831469612302545237078788377618,
    0.707106781186547524400844362105,
    -0.555570233019602224742830813949,
    0.382683432365089771728459984030,
    -0.195090322016128267848284868477,
];

#[inline(always)]
fn idct_1d(m_in: &[f32; 8], m_out: &mut [f32; 8]) {
    const SQRT2_O2: f32 = 0.707106781186547524400844362105;

    for n in 0..8 {
        let mut sum = 0.;
        for k in 0..8 {
            let s = if k == 0 { SQRT2_O2 } else { 1. };
            // TODO: do not always use mul_add,
            // since this generates calls to an fma *function* if fma
            // as an instruction is not available for the target.

            // this is going to slow down the code a lot in some cases,
            // since the codegen on the default x86 target is way worse.

            // sum = f32::mul_add(s, m_in[k] * cos_table(n, k), sum)
            sum += s * m_in[k] * COS_TABLE[8 * n + k];
        }
        m_out[n] = sum * 0.5;
    }
}

pub fn idct(m_in: &[f32; 64], m_out: &mut [f32; 64]) {
    unsafe {
        let mut transposed = [0.; 64];
        transpose8x8(m_in, &mut transposed);

        for i in 0..8 {
            idct_1d(
                cast(&transposed[8 * i..][..8]),
                cast_mut(&mut m_out[8 * i..][..8]),
            );
        }

        transpose8x8(m_out, &mut transposed);

        for i in 0..8 {
            idct_1d(
                cast(&transposed[8 * i..][..8]),
                cast_mut(&mut m_out[8 * i..][..8]),
            );
        }

        // TODO just curious if doing the transpose here
        // also works (and removing the first transpose)
    }
}
