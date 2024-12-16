use lazy_static::lazy_static;

pub const BLACK: i32 = 0;
pub const WHITE: i32 = 1;
pub const EMPTY: i32 = 2;

/// Used in LEVEL table.
#[derive(Clone, Copy)]
pub struct Level {
    /// Search depth
    pub depth: i32,

    /// Selectivity level
    pub selectivity: i32,
}

lazy_static! {
    /// Table that maps engine level and number of empties to search depth and selectivity.
    pub static ref LEVEL: [[Level; 61]; 61] = {
        let mut level_array = [[Level {
            depth: 0,
            selectivity: 0,
        }; 61]; 61];

        for level in 0..=60 {
            for n_empties in 0..=60 {
                level_array[level as usize][n_empties as usize] = if level <= 0 {
                    Level {
                        depth: 0,
                        selectivity: 5,
                    }
                } else if level <= 10 {
                    Level {
                        depth: if n_empties <= 2 * level {
                            n_empties
                        } else {
                            level
                        },
                        selectivity: 5,
                    }
                } else if level <= 12 {
                    match n_empties {
                        n if n <= 21 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 24 => Level {
                            depth: n,
                            selectivity: 3,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level <= 18 {
                    match n_empties {
                        n if n <= 21 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 24 => Level {
                            depth: n,
                            selectivity: 3,
                        },
                        n if n <= 27 => Level {
                            depth: n,
                            selectivity: 1,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level <= 21 {
                    match n_empties {
                        n if n <= 24 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 27 => Level {
                            depth: n,
                            selectivity: 3,
                        },
                        n if n <= 30 => Level {
                            depth: n,
                            selectivity: 1,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level <= 24 {
                    match n_empties {
                        n if n <= 24 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 27 => Level {
                            depth: n,
                            selectivity: 4,
                        },
                        n if n <= 30 => Level {
                            depth: n,
                            selectivity: 2,
                        },
                        n if n <= 33 => Level {
                            depth: n,
                            selectivity: 0,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level <= 27 {
                    match n_empties {
                        n if n <= 27 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 30 => Level {
                            depth: n,
                            selectivity: 3,
                        },
                        n if n <= 33 => Level {
                            depth: n,
                            selectivity: 1,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level <= 31 {
                    match n_empties {
                        n if n <= 30 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 33 => Level {
                            depth: n,
                            selectivity: 3,
                        },
                        n if n <= 36 => Level {
                            depth: n,
                            selectivity: 1,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level <= 33 {
                    match n_empties {
                        n if n <= 30 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 33 => Level {
                            depth: n,
                            selectivity: 4,
                        },
                        n if n <= 36 => Level {
                            depth: n,
                            selectivity: 2,
                        },
                        n if n <= 39 => Level {
                            depth: n,
                            selectivity: 0,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level <= 35 {
                    match n_empties {
                        n if n <= 30 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= 33 => Level {
                            depth: n,
                            selectivity: 4,
                        },
                        n if n <= 36 => Level {
                            depth: n,
                            selectivity: 3,
                        },
                        n if n <= 39 => Level {
                            depth: n,
                            selectivity: 1,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else if level < 60 {
                    match n_empties {
                        n if n <= level - 6 => Level {
                            depth: n,
                            selectivity: 5,
                        },
                        n if n <= level - 3 => Level {
                            depth: n,
                            selectivity: 4,
                        },
                        n if n <= level => Level {
                            depth: n,
                            selectivity: 3,
                        },
                        n if n <= level + 3 => Level {
                            depth: n,
                            selectivity: 2,
                        },
                        n if n <= level + 6 => Level {
                            depth: n,
                            selectivity: 1,
                        },
                        n if n <= level + 9 => Level {
                            depth: n,
                            selectivity: 0,
                        },
                        _ => Level {
                            depth: level,
                            selectivity: 0,
                        },
                    }
                } else {
                    Level {
                        depth: n_empties,
                        selectivity: 5,
                    }
                };
            }
        }
        level_array
    };
}

/// Indicates if a search thread is running, has finished or should stop.
///
/// Like Stop in Edax
#[repr(u8)]
pub enum Stop {
    /// Search is currently running normally
    Running = 0,

    /// Stop signal for parallel search operations
    StopParallelSearch = 1,

    /// Stop analyzing while waiting for opponent's move
    StopPondering = 2,

    /// Stop due to time limit being reached
    StopTimeout = 3,

    /// Stop requested by user or external command
    StopOnDemand = 4,

    /// Search has completed normally
    StopEnd = 5,
}

impl Stop {
    /// Check if the search is running.
    pub fn is_running(&self) -> bool {
        matches!(self, Stop::Running)
    }
}

/// Number of moves in game including passing moves.
pub const GAME_SIZE: usize = 80;

/// Indicates the type of node in a search tree.
#[repr(u8)]
#[derive(Default, Copy, Clone)]
pub enum NodeType {
    #[default]
    PvNode = 0,
    CutNode = 1,
    AllNode = 2,
}

/// Minimum final score
pub const SCORE_MIN: i32 = -64;

/// Maximum final score
pub const SCORE_MAX: i32 = 64;

/// Infinity score
pub const SCORE_INF: i32 = 127;

/// Used in SELECTIVITY_TABLE.
pub struct Selectivity {
    /// Selectivity value
    pub t: f64,

    /// Level of selectivity
    pub level: i32,

    /// Selectivity value as a percentage
    pub percent: i32,
}

/// Table that maps selectivity value to search depth and selectivity.
#[rustfmt::skip]
pub const SELECTIVITY_TABLE: [Selectivity; 6] = [
    Selectivity { t: 1.1, level: 0, percent: 73 }, // strong selectivity
    Selectivity { t: 1.5, level: 1, percent: 87 }, //       |
    Selectivity { t: 2.0, level: 2, percent: 95 }, //       |
    Selectivity { t: 2.6, level: 3, percent: 98 }, //       |
    Selectivity { t: 3.3, level: 4, percent: 99 }, //       V
    Selectivity { t: 999.0, level: 5, percent: 100 }, // no selectivity
];

/// Index in SELECTIVITY_TABLE for no selectivity
pub const NO_SELECTIVITY: i32 = 5;

/// Threshold for switching between midgame and endgame evaluation.
/// When the number of empty squares is less than or equal to this value,
/// the engine switches from using midgame evaluation (heuristic scoring)
/// to endgame evaluation (exact scoring).
pub const ITERATIVE_MIN_EMPTIES: i32 = 10;

/// Delta value used in move sorting to determine the lower bound for move evaluation.
/// When evaluating moves for sorting, we use (alpha - SORT_ALPHA_DELTA) as a threshold
/// to identify promising moves. This helps prune less promising moves early in the search
/// while ensuring we don't miss potentially good moves that are just slightly below alpha.
pub const SORT_ALPHA_DELTA: i32 = 8;

/// Threshold values to try stability cutoff during PVS search.
/// 99 means unused value.
#[rustfmt::skip]
pub const PVS_STABILITY_THRESHOLD: [i32; 56] = [
    99, 99, 99, 99, -2,  0,  2,  4,
     6,  8, 12, 14, 16, 18, 20, 22,
    24, 26, 28, 30, 32, 34, 36, 38,
    40, 40, 42, 42, 44, 44, 46, 46,
    48, 48, 50, 50, 52, 52, 54, 54,
    56, 56, 58, 58, 60, 60, 62, 62,
    99, 99, 99, 99, 99, 99, 99, 99,
];

/// Threshold values to try stability cutoff during NWS search.
/// 99 means unused value.
#[rustfmt::skip]
pub const NWS_STABILITY_THRESHOLD: [i32; 56] = [
    99, 99, 99, 99,  6,  8, 10, 12,
    14, 16, 20, 22, 24, 26, 28, 30,
    32, 34, 36, 38, 40, 42, 44, 46,
    48, 48, 50, 50, 52, 52, 54, 54,
    56, 56, 58, 58, 60, 60, 62, 62,
    64, 64, 64, 64, 64, 64, 64, 64,
    99, 99, 99, 99, 99, 99, 99, 99,
];

/// Switch from midgame to endgame search (faster but less node efficient) at this depth.
pub const DEPTH_MIDGAME_TO_ENDGAME: i32 = 15;

/// Increased sort depth for move sorting
pub const INC_SORT_DEPTH: [i32; 3] = [0, -2, -3];

/** Store bestmoves in the pv_hash up to this height. */
pub const PV_HASH_HEIGHT: i32 = 5;

/// This is `options.probcut_d` in Edax
pub const PROBCUT_D: f64 = 0.25;

/// This is an undocumented constant in Edax, used for search probcut calculations
pub const RCD: f64 = 0.5;

/// Try Enhanced Transposition Cut (ETC) starting from this depth.
pub const ETC_MIN_DEPTH: i32 = 5;

/// Switch from endgame to shallow search (faster but less node efficient) at this depth.
pub const DEPTH_TO_SHALLOW_SEARCH: i32 = 7;

/// Conversion array: neighbour bits.
#[rustfmt::skip]
pub const NEIGHBOUR: [u64; 66] = [
	0x0000000000000302, 0x0000000000000705, 0x0000000000000e0a, 0x0000000000001c14,
	0x0000000000003828, 0x0000000000007050, 0x000000000000e0a0, 0x000000000000c040,
	0x0000000000030203, 0x0000000000070507, 0x00000000000e0a0e, 0x00000000001c141c,
	0x0000000000382838, 0x0000000000705070, 0x0000000000e0a0e0, 0x0000000000c040c0,
	0x0000000003020300, 0x0000000007050700, 0x000000000e0a0e00, 0x000000001c141c00,
	0x0000000038283800, 0x0000000070507000, 0x00000000e0a0e000, 0x00000000c040c000,
	0x0000000302030000, 0x0000000705070000, 0x0000000e0a0e0000, 0x0000001c141c0000,
	0x0000003828380000, 0x0000007050700000, 0x000000e0a0e00000, 0x000000c040c00000,
	0x0000030203000000, 0x0000070507000000, 0x00000e0a0e000000, 0x00001c141c000000,
	0x0000382838000000, 0x0000705070000000, 0x0000e0a0e0000000, 0x0000c040c0000000,
	0x0003020300000000, 0x0007050700000000, 0x000e0a0e00000000, 0x001c141c00000000,
	0x0038283800000000, 0x0070507000000000, 0x00e0a0e000000000, 0x00c040c000000000,
	0x0302030000000000, 0x0705070000000000, 0x0e0a0e0000000000, 0x1c141c0000000000,
	0x3828380000000000, 0x7050700000000000, 0xe0a0e00000000000, 0xc040c00000000000,
	0x0203000000000000, 0x0507000000000000, 0x0a0e000000000000, 0x141c000000000000,
	0x2838000000000000, 0x5070000000000000, 0xa0e0000000000000, 0x40c0000000000000,
	0, 0,
];
