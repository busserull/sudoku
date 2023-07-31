use nu_ansi_term as ansi;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct GridCellOptions([bool; 9]);

impl GridCellOptions {
    fn new() -> Self {
        Self([false; 9])
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
    unique: bool,
    options: [bool; 9],
}

impl GridCell {
    fn new(index: Option<usize>) -> Self {
        if let Some(index) = index {
            let mut options = [false; 9];
            options[index] = true;

            Self {
                unique: true,
                given: true,
                options,
            }
        } else {
            Self {
                unique: false,
                given: false,
                options: [true; 9],
            }
        }
    }

    fn count(&self) -> usize {
        self.options.iter().filter(|&&x| x).count()
    }

    fn options(&self) -> GridCellOptions {
        GridCellOptions(self.options.clone())
    }

    fn value(&self) -> Option<usize> {
        self.unique
            .then(|| self.options.iter().position(|&x| x).unwrap())
    }

    fn set(&mut self, value: usize) {
        self.options = [false; 9];
        self.options[value] = true;
        self.unique = true;
    }

    fn remove(&mut self, options: &GridCellOptions) -> usize {
        if self.unique {
            return 0;
        }

        let mut options_removed = 0;

        for (option, &to_remove) in self.options.iter_mut().zip(options.0.iter()) {
            if to_remove {
                options_removed += *option as usize;
                *option = false;
            }
        }

        if self.count() == 1 {
            self.unique = true;
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
            write!(
                f,
                "{}",
                ansi::Color::Magenta.paint(format!("{}", self.count()))
            )
        }
    }
}

enum GridError {
    Inconsistent,
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

    fn solve(&mut self) -> Result<(), GridError> {
        self.reduce()?;

        if self.is_solved() {
            return Ok(());
        }

        let backtrack = self.clone();
        let (trial_index, options) = self.first_unsolved_cell().unwrap();

        for guess in options {
            self.0 = backtrack.0.clone();
            self.0[trial_index].set(guess);

            if self.solve().is_ok() {
                return Ok(());
            }
        }

        Err(GridError::Inconsistent)
    }

    fn is_solved(&self) -> bool {
        self.0.iter().all(|cell| cell.unique)
    }

    fn first_unsolved_cell(&self) -> Option<(usize, GridCellOptions)> {
        self.0
            .iter()
            .enumerate()
            .find_map(|(index, cell)| (!cell.unique).then(|| (index, cell.options())))
    }

    fn reduce(&mut self) -> Result<(), GridError> {
        loop {
            let mut options_removed = 0;

            for number in 0..9 {
                let boxed = self.reduce_grid_box(number);
                let row = self.reduce_grid_row(number);
                let column = self.reduce_grid_column(number);

                match (boxed, row, column) {
                    (Ok(b), Ok(r), Ok(c)) => options_removed += b + r + c,
                    _ => return Err(GridError::Inconsistent),
                }
            }

            if options_removed == 0 {
                break;
            }
        }

        Ok(())
    }

    fn reduce_grid_box(&mut self, box_number: usize) -> Result<usize, GridError> {
        let offset = (box_number / 3) * 27 + (box_number % 3) * 3;
        let mut indices = [0, 1, 2, 9, 10, 11, 18, 19, 20];

        for index in indices.iter_mut() {
            *index += offset;
        }

        self.remove_options(&indices)
    }

    fn reduce_grid_row(&mut self, row_number: usize) -> Result<usize, GridError> {
        let offset = 9 * row_number;
        let mut indices = [0, 1, 2, 3, 4, 5, 6, 7, 8];

        for index in indices.iter_mut() {
            *index += offset;
        }

        self.remove_options(&indices)
    }

    fn reduce_grid_column(&mut self, column_number: usize) -> Result<usize, GridError> {
        let mut indices = [0, 9, 18, 27, 36, 45, 54, 63, 72];

        for index in indices.iter_mut() {
            *index += column_number;
        }

        self.remove_options(&indices)
    }

    fn remove_options(&mut self, indices: &[usize]) -> Result<usize, GridError> {
        let mut set_options = GridCellOptions::new();

        for &index in indices {
            let value = self.0[index].value();

            if set_options.is_set(value) {
                return Err(GridError::Inconsistent);
            }

            set_options.set(value);
        }

        let mut options_removed = 0;

        for &index in indices {
            options_removed += self.0[index].remove(&set_options);
        }

        Ok(options_removed)
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
    let mut grid = Grid::new("b3");

    if grid.solve().is_ok() {
        println!("{}", grid);
    } else {
        println!("Inconsistent grid");
    }
}
