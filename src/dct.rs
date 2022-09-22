use std::f64::consts::PI;
use std::mem::transmute;

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

pub fn idct(inm: &[f64; 64], outm: &mut [f64; 64]) {
    fn alpha(x: usize) -> f64 {
        if x == 0 {
            1.0 / f64::sqrt(2.0)
        } else {
            1.0
        }
    }

    fn get_px((x, y): (usize, usize), coeffs: &[[f64; 8]; 8]) -> f64 {
        let mut sum = 0.0;

        for u in 0..=7 {
            for v in 0..=7 {
                let uf = u as f64;
                let vf = v as f64;
                sum += alpha(u)
                    * alpha(v)
                    // coords are u, v
                    // u is the x coordinate
                    // v is the y coordinate
                    // so we have to index with v first, to select the y coordinate (row)
                    // then index with u to get the x coordinate
                    * coeffs[v][u]
                    * f64::cos(((2 * x + 1) as f64 * uf * PI) / 16.0)
                    * f64::cos(((2 * y + 1) as f64 * vf * PI) / 16.0);
            }
        }

        0.25 * sum
    }

    for y in 0..8 {
        for x in 0..8 {
            unsafe {
                outm[y * 8 + x] = get_px((x, y), transmute(inm));
            }
        }
    }
}
