use nu_ansi_term as ansi;
use std::fmt;
use std::path::Path;

#[derive(Clone, Copy)]
struct SCell {
    given: bool,
    set: bool,
    options: [bool; 9],
}

impl SCell {
    fn new(index: Option<usize>) -> Self {
        if let Some(index) = index {
            let mut options = [false; 9];
            options[index] = true;

            Self {
                set: true,
                given: true,
                options,
            }
        } else {
            Self {
                set: false,
                given: false,
                options: [true; 9],
            }
        }
    }

    fn count(&self) -> usize {
        self.options.iter().filter(|&&x| x).count()
    }

    fn index(&self) -> Option<usize> {
        self.set
            .then(|| self.options.iter().position(|&x| x).unwrap())
    }

    fn remove(&mut self, index: usize) -> bool {
        let count_before = self.count();

        if !self.set {
            self.options[index] = false;
        }

        if self.count() == 1 {
            self.set = true;
        }

        count_before != self.count()
    }
}

impl fmt::Debug for SCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(index) = self.index() {
            let digit = if self.given {
                ansi::Color::Blue.bold().paint(format!("{}", index + 1))
            } else {
                ansi::Color::Green.paint(format!("{}", index + 1))
            };

            write!(f, "{}", digit)
        } else {
            write!(
                f,
                "{}",
                ansi::Color::Yellow.paint(format!("{}", self.count()))
            )
        }
    }
}

impl fmt::Display for SCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = self
            .index()
            .map(|index| format!("{}", index + 1))
            .unwrap_or_else(|| "_".to_string());

        if self.given {
            write!(f, "{}", ansi::Color::Blue.bold().paint(symbol))
        } else {
            write!(f, "{}", symbol)
        }
    }
}

struct Game {
    grid: [SCell; 81],
}

impl Game {
    fn remove_illegal_options(&mut self) {
        loop {
            let mut options_removed = 0;

            for number in 0..9 {
                options_removed += self.ex_group(number);
                options_removed += self.ex_row(number);
                options_removed += self.ex_column(number);
            }

            if options_removed == 0 {
                break;
            }
        }
    }

    fn ex_group(&mut self, box_number: usize) -> usize {
        let start_index = match box_number {
            0 => 0,
            1 => 3,
            2 => 6,
            3 => 27,
            4 => 30,
            5 => 33,
            6 => 54,
            7 => 57,
            8 => 60,
            _ => unreachable!(),
        };

        let mut indices = [0, 1, 2, 9, 10, 11, 18, 19, 20];

        for index in indices.iter_mut() {
            *index += start_index;
        }

        self.remove_options(&indices)
    }

    fn ex_row(&mut self, row_number: usize) -> usize {
        let start_index = 9 * row_number;

        let mut indices = [0, 1, 2, 3, 4, 5, 6, 7, 8];

        for index in indices.iter_mut() {
            *index += start_index;
        }

        self.remove_options(&indices)
    }

    fn ex_column(&mut self, column_number: usize) -> usize {
        let mut indices = [0, 9, 18, 27, 36, 45, 54, 63, 72];

        for index in indices.iter_mut() {
            *index += column_number;
        }

        self.remove_options(&indices)
    }

    fn remove_options(&mut self, indices: &[usize]) -> usize {
        let mut set_indices = [false; 9];

        for &index in indices.iter() {
            if let Some(index) = self.grid[index].index() {
                set_indices[index] = true;
            }
        }

        let set_indices: Vec<usize> = set_indices
            .iter()
            .enumerate()
            .filter(|&(_, &is_set)| is_set)
            .map(|(index, _)| index)
            .collect();

        let mut options_removed = 0;

        for &index in indices.iter() {
            for &to_remove in set_indices.iter() {
                if self.grid[index].remove(to_remove) {
                    options_removed += 1;
                }
            }
        }

        options_removed
    }
}

impl Game {
    fn new<P: AsRef<Path>>(path: P) -> Self {
        let mut grid = [SCell::new(None); 81];
        let content = std::fs::read_to_string(path).expect("cannot read game file");

        let mut grid_index = 0;
        for ch in content.chars() {
            match ch {
                'x' => grid_index += 1,

                '1'..='9' => {
                    let digit = ch as usize - '0' as usize;
                    grid[grid_index] = SCell::new(Some(digit - 1));
                    grid_index += 1;
                }

                _ => (),
            }
        }

        Game { grid }
    }
}

impl fmt::Debug for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, cell) in self.grid.iter().enumerate() {
            write!(f, " {:?} ", cell)?;

            if index == 80 {
            } else if (index + 1) % 27 == 0 {
                writeln!(f, "\n{}", "-".repeat(33))?;
            } else if (index + 1) % 9 == 0 {
                writeln!(f, "")?;
            } else if (index + 1) % 3 == 0 {
                write!(f, " | ")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, cell) in self.grid.iter().enumerate() {
            write!(f, " {} ", cell)?;

            if index == 80 {
            } else if (index + 1) % 27 == 0 {
                writeln!(f, "\n{}", "-".repeat(33))?;
            } else if (index + 1) % 9 == 0 {
                writeln!(f, "")?;
            } else if (index + 1) % 3 == 0 {
                write!(f, " | ")?;
            }
        }

        Ok(())
    }
}

fn main() {
    let mut game = Game::new("hard");

    println!("Before solving:\n{}\n\n", game);

    game.remove_illegal_options();

    println!("After removing illegal options:\n{:?}", game);
}
