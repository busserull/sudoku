use nu_ansi_term as ansi;
use std::fmt;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Solve a Sudoku puzzle
    Solve {
        /// Sudoku grid file
        grid_file: PathBuf,

        /// Solution search cutoff
        #[arg(short = 's', long, default_value_t = 12)]
        max_solutions: usize,
    },

    /// Generate a Sudoku puzzle
    Make,
}

#[derive(Debug, Clone, Copy)]
struct GridCellOptions([bool; 9]);

impl GridCellOptions {
    fn all() -> Self {
        Self([true; 9])
    }

    fn none() -> Self {
        Self([false; 9])
    }

    fn single(value: usize) -> Self {
        let mut options = [false; 9];
        options[value] = true;

        Self(options)
    }

    fn is_set(&self, value: Option<usize>) -> bool {
        value.map(|value| self.0[value]).unwrap_or(false)
    }

    fn set(&mut self, value: Option<usize>) {
        if let Some(value) = value {
            self.0[value] = true;
        }
    }
}

impl Iterator for GridCellOptions {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(option) = self.0.iter().position(|option| *option) {
            self.0[option] = false;
            Some(option)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
struct GridCell {
    given: bool,
    options: GridCellOptions,
}

impl GridCell {
    fn new(value: Option<usize>) -> Self {
        if let Some(value) = value {
            let options = GridCellOptions::single(value);

            Self {
                given: true,
                options,
            }
        } else {
            Self {
                given: false,
                options: GridCellOptions::all(),
            }
        }
    }

    fn unique(&self) -> bool {
        self.count() == 1
    }

    fn count(&self) -> usize {
        self.options.0.iter().filter(|&&x| x).count()
    }

    fn options(&self) -> GridCellOptions {
        self.options.clone()
    }

    fn value(&self) -> Option<usize> {
        self.unique()
            .then(|| self.options.0.iter().position(|&x| x).unwrap())
    }

    fn set(&mut self, value: usize) {
        self.options = GridCellOptions::single(value);
    }

    fn remove(&mut self, options: &GridCellOptions) -> usize {
        if self.unique() {
            return 0;
        }

        let mut options_removed = 0;

        for (option, &to_remove) in self.options.0.iter_mut().zip(options.0.iter()) {
            if to_remove {
                options_removed += *option as usize;
                *option = false;
            }
        }

        options_removed
    }
}

impl fmt::Display for GridCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(value) = self.value() {
            let digit = if self.given {
                ansi::Color::Blue.bold().paint(format!("{}", value + 1))
            } else {
                ansi::Color::Green.paint(format!("{}", value + 1))
            };

            write!(f, "{}", digit)
        } else {
            write!(f, "\u{25aa}")
        }
    }
}

enum GridDeduction {
    NoChange,
    Consistent,
    Conflict,
}

impl GridDeduction {
    fn is_consistent(&self) -> bool {
        matches!(self, GridDeduction::Consistent)
    }

    fn no_conflict(&self) -> bool {
        !matches!(self, GridDeduction::Conflict)
    }
}

impl std::ops::BitAndAssign for GridDeduction {
    fn bitand_assign(&mut self, rhs: Self) {
        use GridDeduction::*;

        let deduction = match (&self, rhs) {
            (Conflict, _) => Conflict,
            (_, Conflict) => Conflict,

            (Consistent, _) => Consistent,
            (_, Consistent) => Consistent,

            _ => NoChange,
        };

        *self = deduction
    }
}

#[derive(Clone, Copy)]
struct Grid([GridCell; 81]);

impl Grid {
    fn new<P: AsRef<Path>>(path: P) -> Self {
        let mut cells = [GridCell::new(None); 81];
        let file = std::fs::read_to_string(path).expect("cannot read grid file");

        let mut grid_index = 0;
        for ch in file.chars() {
            match ch {
                'x' => grid_index += 1,

                '1'..='9' => {
                    let digit = ch as usize - '0' as usize;
                    cells[grid_index] = GridCell::new(Some(digit - 1));
                    grid_index += 1;
                }

                _ => (),
            }
        }

        Self(cells)
    }

    fn solve(mut self, solutions_cutoff: Option<usize>) -> Vec<Grid> {
        let mut solutions = Vec::new();

        self.find_solutions(&mut solutions, solutions_cutoff);

        solutions
    }

    fn find_solutions(&mut self, solutions: &mut Vec<Grid>, cutoff: Option<usize>) {
        if let Some(max) = cutoff {
            if solutions.len() >= max {
                return;
            }
        }

        if let Some((trial_index, options)) = self.first_unsolved_cell() {
            let backtrack = self.clone();

            for guess in options {
                self.0 = backtrack.0.clone();
                self.0[trial_index].set(guess);

                if self.deduce().no_conflict() {
                    self.find_solutions(solutions, cutoff);
                }
            }
        } else {
            solutions.push(self.clone());
        }
    }

    fn first_unsolved_cell(&self) -> Option<(usize, GridCellOptions)> {
        self.0
            .iter()
            .enumerate()
            .find_map(|(index, cell)| (!cell.unique()).then(|| (index, cell.options())))
    }

    fn deduce(&mut self) -> GridDeduction {
        let mut result = GridDeduction::Consistent;

        while result.is_consistent() {
            result = GridDeduction::NoChange;

            for number in 0..9 {
                result &= self.deduce_box(number);
                result &= self.deduce_row(number);
                result &= self.deduce_column(number);
            }
        }

        result
    }

    fn deduce_box(&mut self, box_number: usize) -> GridDeduction {
        let offset = (box_number / 3) * 27 + (box_number % 3) * 3;
        let mut indices = [0, 1, 2, 9, 10, 11, 18, 19, 20];

        for index in indices.iter_mut() {
            *index += offset;
        }

        self.remove_options(&indices)
    }

    fn deduce_row(&mut self, row_number: usize) -> GridDeduction {
        let offset = 9 * row_number;
        let mut indices = [0, 1, 2, 3, 4, 5, 6, 7, 8];

        for index in indices.iter_mut() {
            *index += offset;
        }

        self.remove_options(&indices)
    }

    fn deduce_column(&mut self, column_number: usize) -> GridDeduction {
        let mut indices = [0, 9, 18, 27, 36, 45, 54, 63, 72];

        for index in indices.iter_mut() {
            *index += column_number;
        }

        self.remove_options(&indices)
    }

    fn remove_options(&mut self, indices: &[usize]) -> GridDeduction {
        let mut set_options = GridCellOptions::none();

        for &index in indices {
            let value = self.0[index].value();

            if set_options.is_set(value) {
                return GridDeduction::Conflict;
            }

            set_options.set(value);
        }

        let options_removed: usize = indices
            .iter()
            .map(|&index| self.0[index].remove(&set_options))
            .sum();

        if options_removed == 0 {
            GridDeduction::NoChange
        } else {
            GridDeduction::Consistent
        }
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\u{250c}{}\u{252c}{}\u{252c}{}\u{2510}",
            "\u{2500}".repeat(10),
            "\u{2500}".repeat(11),
            "\u{2500}".repeat(10)
        )?;

        for (index, cell) in self.0.iter().enumerate() {
            if index % 9 == 0 {
                write!(f, "\u{2502}")?;
            }

            write!(f, " {} ", cell)?;

            if index == 80 {
                write!(f, "\u{2502}")?;
            } else if (index + 1) % 27 == 0 {
                writeln!(
                    f,
                    "\u{2502}\n\u{251c}{}\u{253c}{}\u{253c}{}\u{2524}",
                    "\u{2500}".repeat(10),
                    "\u{2500}".repeat(11),
                    "\u{2500}".repeat(10)
                )?;
            } else if (index + 1) % 9 == 0 {
                writeln!(f, "\u{2502}")?;
            } else if (index + 1) % 3 == 0 {
                write!(f, " \u{2502} ")?;
            }
        }

        write!(
            f,
            "\n\u{2514}{}\u{2534}{}\u{2534}{}\u{2518}",
            "\u{2500}".repeat(10),
            "\u{2500}".repeat(11),
            "\u{2500}".repeat(10)
        )
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Solve {
            grid_file,
            max_solutions,
        } => {
            let grid = Grid::new(grid_file);
            println!("Unsolved:\n{}", grid);

            let solutions = grid.solve(Some(max_solutions));

            for (index, solution) in solutions.iter().enumerate() {
                println!("\nSolution {}:\n{}", index + 1, solution);
            }
        }

        Commands::Make => (),
    }
}
