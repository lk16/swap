use lazy_static::lazy_static;

// TODO move more Edax constants here

pub const BLACK: i32 = 0;
pub const WHITE: i32 = 1;
pub const EMPTY: i32 = 2;

#[derive(Clone, Copy)]
pub struct Level {
    pub depth: i32,
    pub selectivity: i32,
}

lazy_static! {
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

/// Represents different states that can stop or interrupt a search
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
    pub fn is_running(&self) -> bool {
        matches!(self, Stop::Running)
    }
}

/// Number of moves in game including passing moves.
pub const GAME_SIZE: usize = 80;

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
