use std::f64::consts::PI;

#[inline(always)]
unsafe fn cast<const N: usize, T>(x: &[T]) -> &[T; N] {
    &*(x as *const [T] as *const [T; N])
}

#[inline(always)]
unsafe fn cast_mut<const N: usize, T>(x: &mut [T]) -> &mut [T; N] {
    &mut *(x as *mut [T] as *mut [T; N])
}

fn transpose8x8(inm: &[f64; 64], outm: &mut [f64; 64]) {
    for i in 0..8 {
        for j in 0..8 {
            outm[j * 8 + i] = inm[i * 8 + j];
        }
    }
}

fn idct_1d(m_in: &[f64; 8], m_out: &mut [f64; 8]) {
    const SQRT2_O2: f64 = 0.707106781186547524400844362105;

    for n in 0..8 {
        let mut sum = 0.;
        for k in 0..8 {
            let s = if k == 0 { SQRT2_O2 } else { 1. };
            sum = f64::mul_add(
                s,
                m_in[k] * f64::cos(PI * (n as f64 + 0.5) * k as f64 / 8.0),
                sum,
            )
        }
        m_out[n] = sum * 0.5;
    }
}

pub fn idct(m_in: &[f64; 64], m_out: &mut [f64; 64]) {
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
    }
}
