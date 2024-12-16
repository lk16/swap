/// Count how many discs are flipped by playing the last empty square on the board.
/// This is used for endgame evaluation.
///
/// `index` is the index of the last empty square on the board.
/// `player` is the bitset of the player's discs.
///
/// Like count_last_flip() in Edax.
pub fn count_last_flip(index: usize, player: u64) -> usize {
    COUNT_LAST_FLIP[index](player)
}

type CountLastFlipFn = fn(u64) -> usize;

#[rustfmt::skip]
static COUNT_LAST_FLIP: [CountLastFlipFn; 64] = [
	count_last_flip_a1, count_last_flip_b1, count_last_flip_c1, count_last_flip_d1,
	count_last_flip_e1, count_last_flip_f1, count_last_flip_g1, count_last_flip_h1,
	count_last_flip_a2, count_last_flip_b2, count_last_flip_c2, count_last_flip_d2,
	count_last_flip_e2, count_last_flip_f2, count_last_flip_g2, count_last_flip_h2,
	count_last_flip_a3, count_last_flip_b3, count_last_flip_c3, count_last_flip_d3,
	count_last_flip_e3, count_last_flip_f3, count_last_flip_g3, count_last_flip_h3,
	count_last_flip_a4, count_last_flip_b4, count_last_flip_c4, count_last_flip_d4,
	count_last_flip_e4, count_last_flip_f4, count_last_flip_g4, count_last_flip_h4,
	count_last_flip_a5, count_last_flip_b5, count_last_flip_c5, count_last_flip_d5,
	count_last_flip_e5, count_last_flip_f5, count_last_flip_g5, count_last_flip_h5,
	count_last_flip_a6, count_last_flip_b6, count_last_flip_c6, count_last_flip_d6,
	count_last_flip_e6, count_last_flip_f6, count_last_flip_g6, count_last_flip_h6,
	count_last_flip_a7, count_last_flip_b7, count_last_flip_c7, count_last_flip_d7,
	count_last_flip_e7, count_last_flip_f7, count_last_flip_g7, count_last_flip_h7,
	count_last_flip_a8, count_last_flip_b8, count_last_flip_c8, count_last_flip_d8,
	count_last_flip_e8, count_last_flip_f8, count_last_flip_g8, count_last_flip_h8,
];

#[rustfmt::skip]
const COUNT_FLIP_R: [u8; 128] = [
    0,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0,
    8,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0,
   10,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0,
    8,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0,
   12,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0,
    8,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0,
   10,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0,
    8,  0,  2,  0,  4,  0,  2,  0,  6,  0,  2,  0,  4,  0,  2,  0
];

#[rustfmt::skip]
const COUNT_FLIP_2: [u8; 256] = [
    0,  2,  0,  0,  0,  2,  0,  0,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
    4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
    6,  8,  6,  6,  6,  8,  6,  6,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
    4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
    8, 10,  8,  8,  8, 10,  8,  8,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
    4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
    6,  8,  6,  6,  6,  8,  6,  6,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
    4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0
];

#[rustfmt::skip]
const COUNT_FLIP_3: [u8; 256] = [
    0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
    2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
    4,  8,  6,  6,  4,  4,  4,  4,  4,  8,  6,  6,  4,  4,  4,  4,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
    2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
    6, 10,  8,  8,  6,  6,  6,  6,  6, 10,  8,  8,  6,  6,  6,  6,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
    2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
    4,  8,  6,  6,  4,  4,  4,  4,  4,  8,  6,  6,  4,  4,  4,  4,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
    2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0
];

#[rustfmt::skip]
const COUNT_FLIP_4: [u8; 256] = [
    0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
    2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,
    0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
    4, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  4, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,
    0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
    2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,
    0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const COUNT_FLIP_5: [u8; 256] = [
    0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    2, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
    2, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
    0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const COUNT_FLIP_L: [u8; 128] = [
    0, 12, 10, 10,  8,  8,  8,  8,  6,  6,  6,  6,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,
    2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0
];

/// Like count_V_flip_reverse() in Edax
fn count_v_flip_reverse(player: u64, offset: i32) -> usize {
    // TODO #15 further optimization: Edax uses the alternative implementation
    // in count_last_flip_bitscan.c, see if it's faster.

    // Shift player bits left by offset and set least significant bit to 1
    let shifted = (player << offset) | 1;

    // Count leading zeros and add 1
    // Note: leading_zeros() in Rust is equivalent to __builtin_clzll in C
    let count = (shifted.leading_zeros() as usize + 1) >> 2;

    // Apply mask to get final result (equivalent to & 0x0E)
    count & 0x0E
}

fn count_last_flip_a1(p: u64) -> usize {
    let p_v = p & 0x0101010101010100;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x000020406080a0c0)) >> 60;
    n_flipped += COUNT_FLIP_R[((p >> 1) & 0x7f) as usize] as u64;
    let p_d9 = (p & 0x8040201008040200) >> 8;
    n_flipped += ((p_d9 & p_d9.wrapping_neg()).wrapping_mul(0x0008080604028180)) >> 60;

    n_flipped as usize
}

fn count_last_flip_b1(p: u64) -> usize {
    let p_v = p & 0x0202020202020200;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000102030405060)) >> 60;
    n_flipped += COUNT_FLIP_R[((p >> 2) & 0x3f) as usize] as u64;
    let p_d9 = p & 0x0080402010080400;
    n_flipped += ((p_d9 & p_d9.wrapping_neg()).wrapping_mul(0x0000040403020140)) >> 60;

    n_flipped as usize
}

fn count_last_flip_c1(p: u64) -> usize {
    let p_v = p & 0x0404040404040400;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000081018202830)) >> 60;
    n_flipped += COUNT_FLIP_2[(p & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x0000804020110A04).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_d1(p: u64) -> usize {
    let p_v = p & 0x0808080808080800;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x000004080c101418)) >> 60;
    n_flipped += COUNT_FLIP_3[(p & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0000008041221408).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_e1(p: u64) -> usize {
    let p_v = p & 0x1010101010101000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000020406080a0c)) >> 60;
    n_flipped += COUNT_FLIP_4[(p & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0000000182442810).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_f1(p: u64) -> usize {
    let p_v = p & 0x2020202020202000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000010203040506)) >> 60;
    n_flipped += COUNT_FLIP_5[(p & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0000010204885020).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_g1(p: u64) -> usize {
    let p_v = p & 0x4040404040404000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000008101820283)) >> 60;
    n_flipped += COUNT_FLIP_L[((p << 1) & 0x7e) as usize] as u64;
    let p_d7 = p & 0x0001020408102000;
    n_flipped += ((p_d7 & p_d7.wrapping_neg()).wrapping_mul(0x000002081840a000)) >> 60;

    n_flipped as usize
}

fn count_last_flip_h1(p: u64) -> usize {
    let p_v = (p & 0x8080808080808000) >> 1;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000008101820283)) >> 60;
    n_flipped += COUNT_FLIP_L[(p & 0x7f) as usize] as u64;
    let p_d7 = p & 0x0102040810204000;
    n_flipped += ((p_d7 & p_d7.wrapping_neg()).wrapping_mul(0x000001040c2050c0)) >> 60;

    n_flipped as usize
}

fn count_last_flip_a2(p: u64) -> usize {
    let p_v = p & 0x0101010101010000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x00000020406080a0)) >> 60;
    n_flipped += COUNT_FLIP_R[((p >> 9) & 0x7f) as usize] as u64;
    let p_d9 = (p & 0x4020100804020000) >> 8;
    n_flipped += ((p_d9 & p_d9.wrapping_neg()).wrapping_mul(0x0000080806040280)) >> 60;

    n_flipped as usize
}

fn count_last_flip_b2(p: u64) -> usize {
    let p_v = p & 0x0202020202020000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000001020304050)) >> 60;
    n_flipped += COUNT_FLIP_R[((p >> 10) & 0x3f) as usize] as u64;
    let p_d9 = (p & 0x8040201008040000) >> 8;
    n_flipped += ((p_d9 & p_d9.wrapping_neg()).wrapping_mul(0x0000040403020140)) >> 60;

    n_flipped as usize
}

fn count_last_flip_c2(p: u64) -> usize {
    let p_v = p & 0x0404040404040000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000000810182028)) >> 60;
    n_flipped += COUNT_FLIP_2[((p >> 8) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x00804020110A0400).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_d2(p: u64) -> usize {
    let p_v = p & 0x0808080808080000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x00000004080c1014)) >> 60;
    n_flipped += COUNT_FLIP_3[((p >> 8) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0000804122140800).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_e2(p: u64) -> usize {
    let p_v = p & 0x1010101010100000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x000000020406080a)) >> 60;
    n_flipped += COUNT_FLIP_4[((p >> 8) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0000018244281000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_f2(p: u64) -> usize {
    let p_v = p & 0x2020202020200000;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000000102030405)) >> 60;
    n_flipped += COUNT_FLIP_5[((p >> 8) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0001020488502000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_g2(p: u64) -> usize {
    let p_v = (p & 0x4040404040400000) >> 1;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000000102030405)) >> 60;
    n_flipped += COUNT_FLIP_L[((p >> 7) & 0x7e) as usize] as u64;
    let p_d7 = p & 0x0102040810200000;
    n_flipped += ((p_d7 & p_d7.wrapping_neg()).wrapping_mul(0x00000002081840a0)) >> 60;

    n_flipped as usize
}

fn count_last_flip_h2(p: u64) -> usize {
    let p_v = (p & 0x8080808080800000) >> 2;
    let mut n_flipped = ((p_v & p_v.wrapping_neg()).wrapping_mul(0x0000000102030405)) >> 60;
    n_flipped += COUNT_FLIP_L[((p >> 8) & 0x7f) as usize] as u64;
    let p_d7 = (p & 0x0204081020400000) >> 2;
    n_flipped += ((p_d7 & p_d7.wrapping_neg()).wrapping_mul(0x0000000410308143)) >> 60;

    n_flipped as usize
}

fn count_last_flip_a3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x0101010101010101).wrapping_mul(0x0102040810204080)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_R[((p >> 17) & 0x7f) as usize] as u64;
    let p_d9 = p & 0x2010080402000000;
    n_flipped += ((p_d9 & p_d9.wrapping_neg()).wrapping_mul(0x0000000008080604)) >> 60;
    n_flipped += (p >> 1) & (!p >> 8) & 2;

    n_flipped as usize
}

fn count_last_flip_b3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x0202020202020202).wrapping_mul(0x0081020408102040)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_R[((p >> 18) & 0x3f) as usize] as u64;
    let p_d9 = p & 0x4020100804000000;
    n_flipped += ((p_d9 & p_d9.wrapping_neg()).wrapping_mul(0x0000000004040302)) >> 60;
    n_flipped += (p >> 2) & (!p >> 9) & 2;

    n_flipped as usize
}

fn count_last_flip_c3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x0404040404040404).wrapping_mul(0x0040810204081020)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2[((p >> 16) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x0000000102040810).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x8040201008040201).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_d3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x0808080808080808).wrapping_mul(0x0020408102040810)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3[((p >> 16) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0000010204081020).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0080402010080402).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_e3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x1010101010101010).wrapping_mul(0x0010204081020408)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4[((p >> 16) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0001020408102040).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0000804020100804).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_f3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x2020202020202020).wrapping_mul(0x0008102040810204)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5[((p >> 16) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0102040810204080).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0000008040201008).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_g3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x4040404040404040).wrapping_mul(0x0004081020408102)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_L[((p >> 15) & 0x7e) as usize] as u64;
    let p_d7 = p & 0x0204081020000000;
    n_flipped += ((p_d7 & p_d7.wrapping_neg()).wrapping_mul(0x0000000002081840)) >> 60;
    n_flipped += (p >> 3) & (!p >> 12) & 2;

    n_flipped as usize
}

fn count_last_flip_h3(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_2
        [(((p & 0x8080808080808080).wrapping_mul(0x0002040810204081)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_L[((p >> 16) & 0x7f) as usize] as u64;
    let p_d7 = p & 0x0408102040000000;
    n_flipped += ((p_d7 & p_d7.wrapping_neg()).wrapping_mul(0x0000000001040c20)) >> 60;
    n_flipped += (p >> 4) & (!p >> 13) & 2;

    n_flipped as usize
}

fn count_last_flip_a4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x1008040201010101).wrapping_mul(0x0102040808080808)) >> 56) as usize]
        as u64; // A1A4E8
    n_flipped += COUNT_FLIP_R[((p >> 25) & 0x7f) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0101010101020408).wrapping_mul(0x1010101008040201)) >> 56) as usize]
        as u64; // D1A4A8

    n_flipped as usize
}

fn count_last_flip_b4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x2010080402020202).wrapping_mul(0x0081020404040404)) >> 56) as usize]
        as u64; // B1B4F8
    n_flipped += COUNT_FLIP_R[((p >> 26) & 0x3f) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [((((p & 0x0202020202040810) >> 1).wrapping_mul(0x1010101008040201)) >> 56) as usize]
        as u64; // E1B4B8

    n_flipped as usize
}

fn count_last_flip_c4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x0404040404040404).wrapping_mul(0x0040810204081020)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2[((p >> 24) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x0000010204081020).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x4020100804020100).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_d4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x0808080808080808).wrapping_mul(0x0020408102040810)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3[((p >> 24) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0001020408102040).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x8040201008040201).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_e4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x1010101010101010).wrapping_mul(0x0010204081020408)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4[((p >> 24) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0102040810204080).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0080402010080402).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_f4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x2020202020202020).wrapping_mul(0x0008102040810204)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5[((p >> 24) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0204081020408000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0000804020100804).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_g4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x4040404040201008).wrapping_mul(0x0020202020408102)) >> 56) as usize]
        as u64; // D1G4G8
    n_flipped += COUNT_FLIP_L[((p >> 23) & 0x7e) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [((((p & 0x0408102040404040) >> 2).wrapping_mul(0x0804020101010101)) >> 56) as usize]
        as u64; // G1G4C8

    n_flipped as usize
}

fn count_last_flip_h4(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_3
        [(((p & 0x8080808080402010).wrapping_mul(0x0010101010204081)) >> 56) as usize]
        as u64; // E1H4H8
    n_flipped += COUNT_FLIP_L[((p >> 24) & 0x7f) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [((((p & 0x0810204080808080) >> 3).wrapping_mul(0x0804020101010101)) >> 56) as usize]
        as u64; // H1H4D8

    n_flipped as usize
}

fn count_last_flip_a5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x0804020101010101).wrapping_mul(0x0102040810101010)) >> 56) as usize]
        as u64; // A1A5D8
    n_flipped += COUNT_FLIP_R[((p >> 33) & 0x7f) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0101010102040810).wrapping_mul(0x0808080808040201)) >> 56) as usize]
        as u64; // E1A5A8

    n_flipped as usize
}

fn count_last_flip_b5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x1008040202020202).wrapping_mul(0x0081020408080808)) >> 56) as usize]
        as u64; // B1B5E8
    n_flipped += COUNT_FLIP_R[((p >> 34) & 0x3f) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [((((p & 0x0202020204081020) >> 1).wrapping_mul(0x0808080808040201)) >> 56) as usize]
        as u64; // F1B5B8

    n_flipped as usize
}

fn count_last_flip_c5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x0404040404040404).wrapping_mul(0x0040810204081020)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2[((p >> 32) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x0001020408102040).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x2010080402010000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_d5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x0808080808080808).wrapping_mul(0x0020408102040810)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3[((p >> 32) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0102040810204080).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x4020100804020100).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_e5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x1010101010101010).wrapping_mul(0x0010204081020408)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4[((p >> 32) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0204081020408000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x8040201008040201).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_f5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x2020202020202020).wrapping_mul(0x0008102040810204)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5[((p >> 32) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0408102040800000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0080402010080402).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_g5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x4040404020100804).wrapping_mul(0x0040404040408102)) >> 56) as usize]
        as u64; // C1G5G8
    n_flipped += COUNT_FLIP_L[((p >> 31) & 0x7e) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [((((p & 0x0810204040404040) >> 3).wrapping_mul(0x1008040201010101)) >> 56) as usize]
        as u64; // G1G5D8

    n_flipped as usize
}

fn count_last_flip_h5(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_4
        [(((p & 0x8080808040201008).wrapping_mul(0x0020202020204081)) >> 56) as usize]
        as u64; // D1H5H8
    n_flipped += COUNT_FLIP_L[((p >> 32) & 0x7f) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [((((p & 0x1020408080808080) >> 4).wrapping_mul(0x1008040201010101)) >> 56) as usize]
        as u64; // H1H5E8

    n_flipped as usize
}

fn count_last_flip_a6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x0402010101010101).wrapping_mul(0x0102040810202020)) >> 56) as usize]
        as u64; // A1A6C8
    n_flipped += COUNT_FLIP_R[((p >> 41) & 0x7f) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x0101010204081020).wrapping_mul(0x0404040404040201)) >> 56) as usize]
        as u64; // F1A6A8

    n_flipped as usize
}

fn count_last_flip_b6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x0804020202020202).wrapping_mul(0x0081020408101010)) >> 56) as usize]
        as u64; // B1B6D8
    n_flipped += COUNT_FLIP_R[((p >> 42) & 0x3f) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [((((p & 0x0202020408102040) >> 1).wrapping_mul(0x0404040404040201)) >> 56) as usize]
        as u64; // G1B6B8

    n_flipped as usize
}

fn count_last_flip_c6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x0404040404040404).wrapping_mul(0x0040810204081020)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2[((p >> 40) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x0102040810204080).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x1008040201000000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_d6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x0808080808080808).wrapping_mul(0x0020408102040810)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3[((p >> 40) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0204081020408000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x2010080402010000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_e6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x1010101010101010).wrapping_mul(0x0010204081020408)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4[((p >> 40) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0408102040800000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x4020100804020100).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_f6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x2020202020202020).wrapping_mul(0x0008102040810204)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5[((p >> 40) & 0xff) as usize] as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0810204080000000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x8040201008040201).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as u64;

    n_flipped as usize
}

fn count_last_flip_g6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x4040402010080402).wrapping_mul(0x0080808080808102)) >> 56) as usize]
        as u64; // B1G6G8
    n_flipped += COUNT_FLIP_L[((p >> 39) & 0x7e) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [((((p & 0x1020404040404040) >> 4).wrapping_mul(0x2010080402010101)) >> 56) as usize]
        as u64; // G1G6E8

    n_flipped as usize
}

fn count_last_flip_h6(p: u64) -> usize {
    let mut n_flipped = COUNT_FLIP_5
        [(((p & 0x8080804020100804).wrapping_mul(0x0040404040404081)) >> 56) as usize]
        as u64; // C1H6H8
    n_flipped += COUNT_FLIP_L[((p >> 40) & 0x7f) as usize] as u64;
    n_flipped += COUNT_FLIP_2
        [((((p & 0x2040808080808080) >> 5).wrapping_mul(0x2010080402010101)) >> 56) as usize]
        as u64; // H1H6F8

    n_flipped as usize
}

fn count_last_flip_a7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000010101010101, 23);
    n_flipped += COUNT_FLIP_R[((p >> 49) & 0x7f) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0000020408102040, 16);

    n_flipped
}

fn count_last_flip_b7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000020202020202, 22);
    n_flipped += COUNT_FLIP_R[((p >> 50) & 0x3f) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0000040810204080, 15);

    n_flipped
}

fn count_last_flip_c7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000040404040404, 21);
    n_flipped += COUNT_FLIP_2[((p >> 48) & 0xff) as usize] as usize;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x00040A1120408000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_d7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000080808080808, 20);
    n_flipped += COUNT_FLIP_3[((p >> 48) & 0xff) as usize] as usize;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0008142241800000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_e7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000101010101010, 19);
    n_flipped += COUNT_FLIP_4[((p >> 48) & 0xff) as usize] as usize;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x0010284482010000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_f7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000202020202020, 18);
    n_flipped += COUNT_FLIP_5[((p >> 48) & 0xff) as usize] as usize;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0020508804020100).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_g7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000404040404040, 17);
    n_flipped += COUNT_FLIP_L[((p >> 47) & 0x7e) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0000201008040201, 18);

    n_flipped
}

fn count_last_flip_h7(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0000808080808080, 16);
    n_flipped += COUNT_FLIP_L[((p >> 48) & 0x7f) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0000402010080402, 17);

    n_flipped
}

fn count_last_flip_a8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0001010101010101, 15);
    n_flipped += COUNT_FLIP_R[(p >> 57) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0002040810204080, 8);

    n_flipped
}

fn count_last_flip_b8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0002020202020202, 14);
    n_flipped += COUNT_FLIP_R[(p >> 58) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0004081020408000, 7);

    n_flipped
}

fn count_last_flip_c8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0004040404040404, 13);
    n_flipped += COUNT_FLIP_2[(p >> 56) as usize] as usize;
    n_flipped += COUNT_FLIP_2
        [(((p & 0x040A112040800000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_d8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0008080808080808, 12);
    n_flipped += COUNT_FLIP_3[(p >> 56) as usize] as usize;
    n_flipped += COUNT_FLIP_3
        [(((p & 0x0814224180000000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_e8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0010101010101010, 11);
    n_flipped += COUNT_FLIP_4[(p >> 56) as usize] as usize;
    n_flipped += COUNT_FLIP_4
        [(((p & 0x1028448201000000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_f8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0020202020202020, 10);
    n_flipped += COUNT_FLIP_5[(p >> 56) as usize] as usize;
    n_flipped += COUNT_FLIP_5
        [(((p & 0x0050880402010000).wrapping_mul(0x0101010101010101)) >> 56) as usize]
        as usize;

    n_flipped
}

fn count_last_flip_g8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0040404040404040, 9);
    n_flipped += COUNT_FLIP_L[((p >> 55) & 0x7e) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0020100804020100, 10);

    n_flipped
}

fn count_last_flip_h8(p: u64) -> usize {
    let mut n_flipped = count_v_flip_reverse(p & 0x0080808080808080, 8);
    n_flipped += COUNT_FLIP_L[((p >> 56) & 0x7f) as usize] as usize;
    n_flipped += count_v_flip_reverse(p & 0x0040201008040201, 9);

    n_flipped
}

#[cfg(test)]
mod tests {

    use crate::othello::position::Position;

    use super::*;

    #[rustfmt::skip]
    const SIMPLE_COUNT_FLIP: [[u8; 256]; 8] = [
        [
             0,  0,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
             8,  8,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
            10, 10,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
             8,  8,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
            12, 12,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
             8,  8,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
            10, 10,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
             8,  8,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,  6,  6,  0,  0,  2,  2,  0,  0,  4,  4,  0,  0,  2,  2,  0,  0,
        ],
        [
             0,  0,  0,  0,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
             6,  6,  6,  6,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
             8,  8,  8,  8,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
             6,  6,  6,  6,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
            10, 10, 10, 10,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
             6,  6,  6,  6,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
             8,  8,  8,  8,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
             6,  6,  6,  6,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,  4,  4,  4,  4,  0,  0,  0,  0,  2,  2,  2,  2,  0,  0,  0,  0,
        ],
        [
             0,  2,  0,  0,  0,  2,  0,  0,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
             4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
             6,  8,  6,  6,  6,  8,  6,  6,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
             4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
             8, 10,  8,  8,  8, 10,  8,  8,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
             4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
             6,  8,  6,  6,  6,  8,  6,  6,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
             4,  6,  4,  4,  4,  6,  4,  4,  0,  2,  0,  0,  0,  2,  0,  0,  2,  4,  2,  2,  2,  4,  2,  2,  0,  2,  0,  0,  0,  2,  0,  0,
        ],
        [
             0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
             2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
             4,  8,  6,  6,  4,  4,  4,  4,  4,  8,  6,  6,  4,  4,  4,  4,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
             2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
             6, 10,  8,  8,  6,  6,  6,  6,  6, 10,  8,  8,  6,  6,  6,  6,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
             2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
             4,  8,  6,  6,  4,  4,  4,  4,  4,  8,  6,  6,  4,  4,  4,  4,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
             2,  6,  4,  4,  2,  2,  2,  2,  2,  6,  4,  4,  2,  2,  2,  2,  0,  4,  2,  2,  0,  0,  0,  0,  0,  4,  2,  2,  0,  0,  0,  0,
        ],
        [
             0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
             0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
             2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
             4, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  4, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,
             0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
             2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  6,  4,  4,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,
        ],
        [
             0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             2, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             2, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0,  8,  6,  6,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
        ],
        [
             0, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0, 10,  8,  8,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
        ],
        [
             0, 12, 10, 10,  8,  8,  8,  8,  6,  6,  6,  6,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,
             2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0, 12, 10, 10,  8,  8,  8,  8,  6,  6,  6,  6,  6,  6,  6,  6,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,
             2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
             0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
        ],
    ];

    #[rustfmt::skip]
    const MASK_D: [[u64; 64]; 2] = [
        [
            0x0000000000000001, 0x0000000000000102, 0x0000000000010204, 0x0000000001020408,
            0x0000000102040810, 0x0000010204081020, 0x0001020408102040, 0x0102040810204080,
            0x0000000000000102, 0x0000000000010204, 0x0000000001020408, 0x0000000102040810,
            0x0000010204081020, 0x0001020408102040, 0x0102040810204080, 0x0204081020408000,
            0x0000000000010204, 0x0000000001020408, 0x0000000102040810, 0x0000010204081020,
            0x0001020408102040, 0x0102040810204080, 0x0204081020408000, 0x0408102040800000,
            0x0000000001020408, 0x0000000102040810, 0x0000010204081020, 0x0001020408102040,
            0x0102040810204080, 0x0204081020408000, 0x0408102040800000, 0x0810204080000000,
            0x0000000102040810, 0x0000010204081020, 0x0001020408102040, 0x0102040810204080,
            0x0204081020408000, 0x0408102040800000, 0x0810204080000000, 0x1020408000000000,
            0x0000010204081020, 0x0001020408102040, 0x0102040810204080, 0x0204081020408000,
            0x0408102040800000, 0x0810204080000000, 0x1020408000000000, 0x2040800000000000,
            0x0001020408102040, 0x0102040810204080, 0x0204081020408000, 0x0408102040800000,
            0x0810204080000000, 0x1020408000000000, 0x2040800000000000, 0x4080000000000000,
            0x0102040810204080, 0x0204081020408000, 0x0408102040800000, 0x0810204080000000,
            0x1020408000000000, 0x2040800000000000, 0x4080000000000000, 0x8000000000000000
        ],
        [
            0x8040201008040201, 0x0080402010080402, 0x0000804020100804, 0x0000008040201008,
            0x0000000080402010, 0x0000000000804020, 0x0000000000008040, 0x0000000000000080,
            0x4020100804020100, 0x8040201008040201, 0x0080402010080402, 0x0000804020100804,
            0x0000008040201008, 0x0000000080402010, 0x0000000000804020, 0x0000000000008040,
            0x2010080402010000, 0x4020100804020100, 0x8040201008040201, 0x0080402010080402,
            0x0000804020100804, 0x0000008040201008, 0x0000000080402010, 0x0000000000804020,
            0x1008040201000000, 0x2010080402010000, 0x4020100804020100, 0x8040201008040201,
            0x0080402010080402, 0x0000804020100804, 0x0000008040201008, 0x0000000080402010,
            0x0804020100000000, 0x1008040201000000, 0x2010080402010000, 0x4020100804020100,
            0x8040201008040201, 0x0080402010080402, 0x0000804020100804, 0x0000008040201008,
            0x0402010000000000, 0x0804020100000000, 0x1008040201000000, 0x2010080402010000,
            0x4020100804020100, 0x8040201008040201, 0x0080402010080402, 0x0000804020100804,
            0x0201000000000000, 0x0402010000000000, 0x0804020100000000, 0x1008040201000000,
            0x2010080402010000, 0x4020100804020100, 0x8040201008040201, 0x0080402010080402,
            0x0100000000000000, 0x0201000000000000, 0x0402010000000000, 0x0804020100000000,
            0x1008040201000000, 0x2010080402010000, 0x4020100804020100, 0x8040201008040201
        ]
    ];

    /// Adapted from last_flip() in count_last_flip_plain.c of Edax
    fn count_last_flip_simple(pos: usize, player: u64) -> usize {
        let x = pos & 7;
        let y = pos >> 3;

        let mut n_flipped = SIMPLE_COUNT_FLIP[y][((((player >> x) & 0x0101010101010101)
            .wrapping_mul(0x0102040810204080))
            >> 56) as usize];
        n_flipped += SIMPLE_COUNT_FLIP[x][((player >> (y * 8)) & 0xFF) as usize];
        n_flipped += SIMPLE_COUNT_FLIP[x]
            [(((player & MASK_D[0][pos]).wrapping_mul(0x0101010101010101)) >> 56) as usize];
        n_flipped += SIMPLE_COUNT_FLIP[x]
            [(((player & MASK_D[1][pos]).wrapping_mul(0x0101010101010101)) >> 56) as usize];

        n_flipped as usize
    }

    #[test]
    fn test_count_last_flip() {
        let mut count = 0;

        while count < 10000 {
            let position = Position::new_random_with_discs(63);

            let moves = position.get_moves();

            if moves == 0 {
                continue;
            }

            // There should be only one move
            assert_eq!(moves.count_ones(), 1);

            let pos = moves.trailing_zeros() as usize;

            // The move should be on an empty square
            assert_eq!((position.player | position.opponent) & (1 << pos), 0);

            let last_flip_simple = count_last_flip_simple(pos, position.player);
            let last_flip = count_last_flip(pos, position.player);

            let get_flipped = 2 * position.get_flipped(pos).count_ones();

            if last_flip_simple != last_flip {
                println!("position:");
                println!("{}", position);

                println!("pos: {}", pos);
                println!();

                println!("count_last_flip_simple: {}", last_flip_simple);
                println!("count_last_flip: {}", last_flip);
                println!("get_flipped: {}", get_flipped);

                panic!();
            }

            count += 1;
        }
    }
}
