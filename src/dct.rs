#[inline(always)]
unsafe fn cast<const N: usize, T>(x: &[T]) -> &[T; N] {
    &*(x as *const [T] as *const [T; N])
}

#[inline(always)]
unsafe fn cast_mut<const N: usize, T>(x: &mut [T]) -> &mut [T; N] {
    &mut *(x as *mut [T] as *mut [T; N])
}

fn transpose8x8(inm: &[i32; 64], outm: &mut [i32; 64]) {
    for i in 0..8 {
        for j in 0..8 {
            outm[j * 8 + i] = inm[i * 8 + j];
        }
    }
}

// see dct.py to see how this table is generated
const COS_TABLE_INT: [i32; 64] = [
    741455, 1028428, 968758, 871859, 741455, 582558, 401273, 204567, 741455, 871859, 401273,
    -204567, -741455, -1028428, -968758, -582558, 741455, 582558, -401273, -1028428, -741455,
    204567, 968758, 871859, 741455, 204567, -968758, -582558, 741455, 871859, -401273, -1028428,
    741455, -204567, -968758, 582558, 741455, -871859, -401273, 1028428, 741455, -582558, -401273,
    1028428, -741455, -204567, 968758, -871859, 741455, -871859, 401273, 204567, -741455, 1028428,
    -968758, 582558, 741455, -1028428, 968758, -871859, 741455, -582558, 401273, -204567,
];

fn idct_1d(m_in: &[i32; 8], m_out: &mut [i32; 8]) {
    for n in 0..8 {
        let mut sum = 0;
        for k in 0..8 {
            sum += (m_in[k] * COS_TABLE_INT[8 * n + k]) >> 20;
        }
        // also try / 2 for i32
        m_out[n] = sum >> 1;
    }
}

pub fn idct(m_in: &[i32; 64], m_out: &mut [i32; 64]) {
    unsafe {
        let mut transposed = [0; 64];
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
