use lazy_static::lazy_static;
use std::io::Read;

const EDAX: i32 = 0x58414445; // "EDAX" in ASCII/hex
const XADE: i32 = 0x45444158; // "XADE" in ASCII/hex (byte-swapped EDAX)
const EVAL: i32 = 0x4C415645; // "EVAL" in ASCII/hex
const LAVE: i32 = 0x4556414C; // "LAVE" in ASCII/hex (byte-swapped EVAL)

/** number of (unpacked) weights */
const EVAL_N_WEIGHT: usize = 226315;

/** number of plies */
const EVAL_N_PLY: usize = 61;

/** feature size */
const EVAL_SIZE: [usize; 13] = [
    19683, 59049, 59049, 59049, 6561, 6561, 6561, 6561, 2187, 729, 243, 81, 1,
];

/** packed feature size */
const EVAL_PACKED_SIZE: [usize; 13] = [
    10206, 29889, 29646, 29646, 3321, 3321, 3321, 3321, 1134, 378, 135, 45, 1,
];

// feature symmetry packing
lazy_static! {
    static ref EVAL_C10: [[usize; 59049]; 2] = {
        let mut eval_c10 = [[0; 59049]; 2];
        let mut t = [0; 59049];
        let mut n = 0;

        for l in 0..59049 {
            let k = ((l / 19683) % 3)
                + ((l / 6561) % 3) * 3
                + ((l / 2187) % 3) * 9
                + ((l / 729) % 3) * 27
                + ((l / 243) % 3) * 243
                + ((l / 81) % 3) * 81
                + ((l / 27) % 3) * 729
                + ((l / 9) % 3) * 2187
                + ((l / 3) % 3) * 6561
                + (l % 3) * 19683;

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_c10[0][l] = t[l];
            eval_c10[1][opponent_feature(l, 10)] = t[l];
        }

        eval_c10
    };
}

lazy_static! {
    static ref EVAL_S10: [[usize; 59049]; 2] = {
        let mut eval_s10 = [[0; 59049]; 2];
        let mut t = [0; 59049];
        let mut n = 0;

        for l in 0..59049 {
            let k = ((l / 19683) % 3)
                + ((l / 6561) % 3) * 3
                + ((l / 2187) % 3) * 9
                + ((l / 729) % 3) * 27
                + ((l / 243) % 3) * 81
                + ((l / 81) % 3) * 243
                + ((l / 27) % 3) * 729
                + ((l / 9) % 3) * 2187
                + ((l / 3) % 3) * 6561
                + (l % 3) * 19683;

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_s10[0][l] = t[l];
            eval_s10[1][opponent_feature(l, 10)] = t[l];
        }

        eval_s10
    };
}

lazy_static! {
    static ref EVAL_C9: [[usize; 19683]; 2] = {
        let mut eval_c9 = [[0; 19683]; 2];
        let mut t = [0; 19683];
        let mut n = 0;

        for l in 0..19683 {
            let k = ((l / 6561) % 3) * 6561
                + ((l / 729) % 3) * 2187
                + ((l / 2187) % 3) * 729
                + ((l / 243) % 3) * 243
                + ((l / 27) % 3) * 81
                + ((l / 81) % 3) * 27
                + ((l / 3) % 3) * 9
                + ((l / 9) % 3) * 3
                + (l % 3);

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_c9[0][l] = t[l];
            eval_c9[1][opponent_feature(l, 9)] = t[l];
        }

        eval_c9
    };
}

lazy_static! {
    static ref EVAL_S8: [[usize; 6561]; 2] = {
        let mut eval_s8 = [[0; 6561]; 2];
        let mut t = [0; 6561];
        let mut n = 0;

        for l in 0..6561 {
            let k = ((l / 2187) % 3)
                + ((l / 729) % 3) * 3
                + ((l / 243) % 3) * 9
                + ((l / 81) % 3) * 27
                + ((l / 27) % 3) * 81
                + ((l / 9) % 3) * 243
                + ((l / 3) % 3) * 729
                + (l % 3) * 2187;

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_s8[0][l] = t[l];
            eval_s8[1][opponent_feature(l, 8)] = t[l];
        }

        eval_s8
    };
}

lazy_static! {
    static ref EVAL_S7: [[usize; 2187]; 2] = {
        let mut eval_s7 = [[0; 2187]; 2];
        let mut t = [0; 2187];
        let mut n = 0;

        for l in 0..2187 {
            let k = ((l / 729) % 3)
                + ((l / 243) % 3) * 3
                + ((l / 81) % 3) * 9
                + ((l / 27) % 3) * 27
                + ((l / 9) % 3) * 81
                + ((l / 3) % 3) * 243
                + (l % 3) * 729;

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_s7[0][l] = t[l];
            eval_s7[1][opponent_feature(l, 7)] = t[l];
        }

        eval_s7
    };
}

lazy_static! {
    static ref EVAL_S6: [[usize; 729]; 2] = {
        let mut eval_s6 = [[0; 729]; 2];
        let mut t = [0; 729];
        let mut n = 0;

        for l in 0..729 {
            let k = ((l / 243) % 3)
                + ((l / 81) % 3) * 3
                + ((l / 27) % 3) * 9
                + ((l / 9) % 3) * 27
                + ((l / 3) % 3) * 81
                + (l % 3) * 243;

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_s6[0][l] = t[l];
            eval_s6[1][opponent_feature(l, 6)] = t[l];
        }

        eval_s6
    };
}

lazy_static! {
    static ref EVAL_S5: [[usize; 243]; 2] = {
        let mut eval_s5 = [[0; 243]; 2];
        let mut t = [0; 243];
        let mut n = 0;

        for l in 0..243 {
            let k = ((l / 81) % 3)
                + ((l / 27) % 3) * 3
                + ((l / 9) % 3) * 9
                + ((l / 3) % 3) * 27
                + (l % 3) * 81;

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_s5[0][l] = t[l];
            eval_s5[1][opponent_feature(l, 5)] = t[l];
        }

        eval_s5
    };
}

lazy_static! {
    static ref EVAL_S4: [[usize; 81]; 2] = {
        let mut eval_s4 = [[0; 81]; 2];
        let mut t = [0; 81];
        let mut n = 0;

        for l in 0..81 {
            let k = ((l / 27) % 3) + ((l / 9) % 3) * 3 + ((l / 3) % 3) * 9 + (l % 3) * 27;

            t[l] = if k < l {
                t[k]
            } else {
                let curr = n;
                n += 1;
                curr
            };
            eval_s4[0][l] = t[l];
            eval_s4[1][opponent_feature(l, 4)] = t[l];
        }

        eval_s4
    };
}

lazy_static! {
    pub static ref EVAL_WEIGHT: Vec<Vec<Vec<i16>>> = load_eval().unwrap();
}

pub fn load_eval() -> Result<Vec<Vec<Vec<i16>>>, std::io::Error> {
    let mut file = std::fs::File::open("eval.dat")?;

    // Read headers
    let mut edax_header = [0u8; 4];
    let mut eval_header = [0u8; 4];
    file.read_exact(&mut edax_header)?;
    file.read_exact(&mut eval_header)?;

    let edax_header = i32::from_le_bytes(edax_header);
    let eval_header = i32::from_le_bytes(eval_header);

    // Validate headers
    if !((edax_header == EDAX && eval_header == EVAL)
        || (edax_header == XADE && eval_header == LAVE))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Not an Edax evaluation file",
        ));
    }

    // Read version info
    let mut version_bytes = [0u8; 4];
    let mut release_bytes = [0u8; 4];
    let mut build_bytes = [0u8; 4];
    let mut date_bytes = [0u8; 8];

    file.read_exact(&mut version_bytes)?;
    file.read_exact(&mut release_bytes)?;
    file.read_exact(&mut build_bytes)?;
    file.read_exact(&mut date_bytes)?;

    let mut _version = i32::from_be_bytes(version_bytes);
    let mut _release = i32::from_be_bytes(release_bytes);
    let mut _build = i32::from_be_bytes(build_bytes);

    // Byte swap if needed
    if edax_header == XADE {
        _version = _version.swap_bytes();
        _release = _release.swap_bytes();
        _build = _build.swap_bytes();
    }

    // Create buffer for weights
    let n_w = EVAL_PACKED_SIZE.iter().sum();
    let mut w = vec![0i16; n_w];

    // Create 3D array for weights directly on heap
    let mut eval_weight = vec![vec![vec![0i16; EVAL_N_WEIGHT]; EVAL_N_PLY]; 2];

    // Load weights for each ply
    for ply in 0..EVAL_N_PLY {
        // Read weights
        let mut bytes = vec![0u8; n_w * 2];
        file.read_exact(&mut bytes)?;

        // Convert bytes to i16 weights
        for i in 0..n_w {
            let val = i16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
            w[i] = if edax_header == XADE {
                val.swap_bytes()
            } else {
                val
            };
        }

        let mut j = 0;
        let mut offset = 0;

        // Process each feature type
        for i in 0..13 {
            let size = EVAL_SIZE[i];

            // Map weights using the appropriate eval array
            match i {
                0 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_C9[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_C9[1][k] + offset];
                        j += 1;
                    }
                }
                1 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_C10[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_C10[1][k] + offset];
                        j += 1;
                    }
                }
                2 | 3 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_S10[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_S10[1][k] + offset];
                        j += 1;
                    }
                }
                4..=7 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_S8[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_S8[1][k] + offset];
                        j += 1;
                    }
                }
                8 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_S7[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_S7[1][k] + offset];
                        j += 1;
                    }
                }
                9 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_S6[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_S6[1][k] + offset];
                        j += 1;
                    }
                }
                10 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_S5[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_S5[1][k] + offset];
                        j += 1;
                    }
                }
                11 => {
                    for k in 0..size {
                        eval_weight[0][ply][j] = w[EVAL_S4[0][k] + offset];
                        eval_weight[1][ply][j] = w[EVAL_S4[1][k] + offset];
                        j += 1;
                    }
                }
                12 => {
                    eval_weight[0][ply][j] = w[offset];
                    eval_weight[1][ply][j] = w[offset];
                }
                _ => unreachable!(),
            }
            offset += EVAL_PACKED_SIZE[i];
        }
    }

    Ok(eval_weight)
}

fn opponent_feature(feature: usize, feature_size: usize) -> usize {
    let f = match feature % 3 {
        0 => 1,
        1 => 0,
        2 => 2,
        _ => unreachable!(),
    };

    if feature_size > 1 {
        f + opponent_feature(feature / 3, feature_size - 1) * 3
    } else {
        f
    }
}
